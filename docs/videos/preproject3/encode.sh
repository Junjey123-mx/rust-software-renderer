#!/usr/bin/env bash
# Regenerates the Camino al Proyecto 3 evidence from the renderer: the
# scripted solar flight (MP4 + GIF) and eight stills.
#
#   docs/videos/preproject3/encode.sh all      flight video and stills
#   docs/videos/preproject3/encode.sh flight   01-solar-flight.mp4 / .gif
#   docs/videos/preproject3/encode.sh stills   01-08 PNG captures
#
# Run from the repository root after `cargo build --release`. Derived from
# docs/videos/lab6/encode.sh: frames come from `--scene solar --record`
# (raw rgb24 at 800x600, exact simulated time per frame) through a lossless
# FFV1 intermediate; the GIF is made from that intermediate and never
# depends on H.264. The H.264 encoder is chosen by a functional test (encode
# one second, check the codec, decode it) in this order: libx264,
# h264_nvenc, h264_vaapi, h264_qsv, libopenh264; H264_ENCODER (and
# VAAPI_DEVICE) may be given in the environment. Without any working
# encoder the script stops, unless ALLOW_GIF_ONLY=1. Every file is written
# to a temporary directory and checked (codec, size, frame count, full
# decode) before it replaces the previous evidence in OUT or SHOTS.
set -euo pipefail

BIN="${BIN:-./target/release/rust-software-renderer}"
OUT="${OUT:-docs/videos/preproject3}"
SHOTS="${SHOTS:-docs/screenshots/preproject3}"
FPS=30
GIF_FPS="${GIF_FPS:-8}"
QP="${QP:-32}"
FLIGHT_SECONDS=22
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

die() { echo "encode.sh: $*" >&2; exit 1; }
[ -x "$BIN" ] || die "renderer not found at $BIN (run cargo build --release)"
mkdir -p "$OUT" "$SHOTS"

stream() {
  ffprobe -v error -count_frames -select_streams v:0 -show_entries "stream=$2" -of csv=p=0 "$1"
}
check() {
  local file="$1" want="$2" got
  [ -s "$file" ] || die "$file is empty"
  got=$(stream "$file" "$3")
  [ "$got" = "$want" ] || die "$file has $got (expected $want)"
  [ -z "$(ffmpeg -v error -i "$file" -f null - 2>&1)" ] || die "$file does not decode cleanly"
}

FILTERS="$(ffmpeg -hide_banner -filters 2>/dev/null)"
for f in palettegen paletteuse select; do grep -q " $f " <<<"$FILTERS" || die "ffmpeg lacks the $f filter"; done

encoder_options() {
  case "$1" in
    libx264) echo "-c:v libx264 -preset medium -crf ${CRF:-28} -pix_fmt yuv420p" ;;
    # Constant QP without B-frames or adaptive quantization (see lab 6).
    h264_nvenc) echo "-c:v h264_nvenc -rc constqp -qp $QP -bf 0 -spatial-aq 0 -temporal-aq 0 -pix_fmt yuv420p" ;;
    h264_vaapi) echo "-vf format=nv12,hwupload -c:v h264_vaapi -qp $QP" ;;
    h264_qsv) echo "-c:v h264_qsv -global_quality $QP -pix_fmt nv12" ;;
    libopenh264) echo "-c:v libopenh264 -b:v ${BITRATE:-700k} -pix_fmt yuv420p" ;;
    *) return 1 ;;
  esac
}
pre_for() { if [ "$1" = h264_vaapi ]; then echo "-vaapi_device ${2:-/dev/dri/renderD128}"; fi; }

probe() {
  local enc="$1" dev="${2:-}" out="$tmp/probe-$1.mp4"
  # shellcheck disable=SC2046
  ffmpeg -hide_banner -v error $(pre_for "$enc" "$dev") -f lavfi -i testsrc2=size=320x240:rate=30 -t 1 \
    $(encoder_options "$enc") -y "$out" 2>/dev/null &&
    [ "$(ffprobe -v error -select_streams v:0 -show_entries stream=codec_name -of csv=p=0 "$out")" = h264 ] &&
    ffmpeg -v error -i "$out" -f null - 2>/dev/null
}

choose_encoder() {
  ENCODER=""
  if [ -n "${H264_ENCODER:-}" ]; then
    probe "$H264_ENCODER" "${VAAPI_DEVICE:-}" && ENCODER="$H264_ENCODER" ||
      echo "encode.sh: H264_ENCODER=$H264_ENCODER failed the functional test" >&2
  fi
  if [ -z "$ENCODER" ]; then
    for enc in libx264 h264_nvenc h264_vaapi h264_qsv libopenh264; do
      if [ "$enc" = h264_vaapi ]; then
        for dev in /dev/dri/renderD*; do
          [ -e "$dev" ] && probe h264_vaapi "$dev" && { ENCODER=h264_vaapi; VAAPI_DEVICE="$dev"; break; }
        done
        [ -n "$ENCODER" ] && break
      elif probe "$enc"; then
        ENCODER="$enc"; break
      fi
    done
  fi
  if [ -z "$ENCODER" ]; then
    [ "${ALLOW_GIF_ONLY:-0}" = 1 ] || die "no working H.264 encoder (set ALLOW_GIF_ONLY=1 for GIF only)"
    echo "encode.sh: no working H.264 encoder, producing GIF only" >&2
  fi
  echo "encode.sh: H.264 encoder = ${ENCODER:-none (GIF only)}" >&2
}

# record <seconds> <fps> <file.mkv> <renderer args...>: lossless intermediate.
record() {
  local seconds="$1" fps="$2" file="$3"; shift 3
  "$BIN" --scene solar --record "$seconds" --fps "$fps" "$@" 2>/dev/null |
    ffmpeg -v error -f rawvideo -pix_fmt rgb24 -s 800x600 -framerate "$fps" -i - \
      -c:v ffv1 -level 3 -y "$file"
  local want frames
  want=$(awk -v s="$seconds" -v f="$fps" 'BEGIN { x = s * f; print (x == int(x)) ? x : int(x) + 1 }')
  frames=$(stream "$file" nb_read_frames)
  [ "$frames" = "$want" ] || die "intermediate $file has $frames frames (expected $want)"
}

# still <name> <intermediate> <frame index>: one PNG, checked, then moved.
still() {
  local name="$1" src="$2" index="$3"
  ffmpeg -v error -i "$src" -vf "select=eq(n\\,$index)" -frames:v 1 -y "$tmp/$name.png"
  check "$tmp/$name.png" "png,800,600" codec_name,width,height
  mv -f "$tmp/$name.png" "$SHOTS/$name.png"
  echo "encode.sh: wrote $SHOTS/$name.png (frame $index)" >&2
}

flight_intermediate() {
  [ -s "$tmp/flight.mkv" ] || record "$FLIGHT_SECONDS" "$FPS" "$tmp/flight.mkv" --timeline flight
}

flight() {
  choose_encoder
  flight_intermediate
  local name=01-solar-flight src="$tmp/flight.mkv" size frames gif_frames
  size=$(stream "$src" width,height)
  frames=$(stream "$src" nb_read_frames)
  if [ -n "$ENCODER" ]; then
    # shellcheck disable=SC2046
    ffmpeg -v error $(pre_for "$ENCODER" "${VAAPI_DEVICE:-}") -i "$src" $(encoder_options "$ENCODER") \
      -r "$FPS" -movflags +faststart -y "$tmp/$name.mp4"
    check "$tmp/$name.mp4" "h264,$size,$FPS/1,$frames" codec_name,width,height,r_frame_rate,nb_read_frames
  fi
  ffmpeg -v error -i "$src" -vf "fps=$GIF_FPS,scale=320:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff:max_colors=64[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
    -loop 0 -y "$tmp/$name.gif"
  gif_frames=$(stream "$tmp/$name.gif" nb_read_frames)
  [ "$gif_frames" -gt 1 ] || die "$name.gif is not animated"
  check "$tmp/$name.gif" "gif,320,$gif_frames" codec_name,width,nb_read_frames
  if [ -n "$ENCODER" ]; then mv -f "$tmp/$name.mp4" "$OUT/$name.mp4"; fi
  mv -f "$tmp/$name.gif" "$OUT/$name.gif"
  echo "encode.sh: wrote $name: $frames frames, GIF $gif_frames frames (${ENCODER:-gif only})" >&2
}

stills() {
  # 01: the orbiting system through the orthographic orbit camera, t = 6 s.
  record 0.04 25 "$tmp/system.mkv" --camera orbit --time 6
  still 01-system-orbit "$tmp/system.mkv" 0
  # 02: the follow camera behind the spacecraft at its start, t = 4 s.
  record 0.04 25 "$tmp/follow.mkv" --time 4
  still 02-ship-follow "$tmp/follow.mkv" 0
  # 03 and 05: frames of the scripted flight (local flight over Crimson
  # Entrance at 8 s; just after the warp to House of Cards at 11.2 s).
  flight_intermediate
  still 03-local-flight "$tmp/flight.mkv" $((8 * FPS))
  # 04: the default Red-Black sky (nebula, star classes, a comet) with
  # another seed, so it differs from 02, t = 14 s.
  record 0.04 25 "$tmp/sky.mkv" --time 14 --star-seed 7
  still 04-skybox "$tmp/sky.mkv" 0
  still 05-quick-travel "$tmp/flight.mkv" $((11 * FPS + 6))
  # 06-08: the royal crown and its loops with idle exhaust (2.1 s), the ring
  # of diamond cards around House of Cards (11.5 s) and the blue afterburner
  # at the peak of the light-speed burst (1.43 s).
  still 06-royal-crown "$tmp/flight.mkv" 63
  still 07-diamond-ring "$tmp/flight.mkv" 345
  still 08-afterburner "$tmp/flight.mkv" 43
}

case "${1:-all}" in
  all) flight; stills ;;
  flight) flight ;;
  stills) stills ;;
  *) die "usage: $0 [all|flight|stills]" ;;
esac

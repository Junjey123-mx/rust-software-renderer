#!/usr/bin/env bash
# Regenerates the Project 3 delivery clips from the renderer, continuous
# tours with no quick travel, no camera switch and no cut.
#
#   docs/videos/project3/encode.sh            01-final-tour.mp4 / .gif
#   docs/videos/project3/encode.sh final      the same (`--timeline final-tour`:
#                                             space flight, approach, a full lap
#                                             of local flight around Crimson
#                                             Entrance, fly-bys of the clover and
#                                             Hearts, boosted departure)
#   docs/videos/project3/encode.sh tour       00-provisional-tour.mp4
#                                             (`--timeline tour`)
#
# Run from the repository root after `cargo build --release`. Derived from
# docs/videos/preproject3/encode.sh: frames come from `--scene solar
# --record` (raw rgb24 at 800x600, exact simulated time per frame) through a
# lossless FFV1 intermediate, and the H.264 encoder is chosen by the same
# functional test (libx264, h264_nvenc, h264_vaapi, h264_qsv, libopenh264;
# H264_ENCODER and VAAPI_DEVICE may be given). The file is written to a
# temporary directory and checked (codec, size, frame rate, frame count,
# full decode, 20 to 60 s for the final tour) before it replaces the
# previous clip in OUT. The GIF is made from the same intermediate.
set -euo pipefail

BIN="${BIN:-./target/release/rust-software-renderer}"
OUT="${OUT:-docs/videos/project3}"
FPS=30
QP="${QP:-32}"
# TOUR_SECONDS and FINAL_TOUR_SECONDS in src/solar/record.rs.
TOUR_SECONDS="${TOUR_SECONDS:-32}"
FINAL_SECONDS="${FINAL_SECONDS:-56}"
GIF_FPS="${GIF_FPS:-6}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

die() { echo "encode.sh: $*" >&2; exit 1; }
[ -x "$BIN" ] || die "renderer not found at $BIN (run cargo build --release)"
mkdir -p "$OUT"

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
for f in null palettegen paletteuse; do grep -q " $f " <<<"$FILTERS" || die "ffmpeg lacks the $f filter"; done

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
  [ -n "$ENCODER" ] || die "no working H.264 encoder"
  echo "encode.sh: H.264 encoder = $ENCODER" >&2
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

# clip <timeline> <seconds> <name> [gif]: H.264 clip (and GIF) of a timeline.
clip() {
  local timeline="$1" seconds="$2" name="$3" gif="${4:-}"
  choose_encoder
  record "$seconds" "$FPS" "$tmp/$timeline.mkv" --timeline "$timeline"
  local src="$tmp/$timeline.mkv" size frames duration gif_frames
  size=$(stream "$src" width,height)
  frames=$(stream "$src" nb_read_frames)
  # shellcheck disable=SC2046
  ffmpeg -v error $(pre_for "$ENCODER" "${VAAPI_DEVICE:-}") -i "$src" $(encoder_options "$ENCODER") \
    -r "$FPS" -movflags +faststart -y "$tmp/$name.mp4"
  check "$tmp/$name.mp4" "h264,$size,$FPS/1,$frames" codec_name,width,height,r_frame_rate,nb_read_frames
  duration=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$tmp/$name.mp4")
  if [ -n "$gif" ]; then
    awk -v d="$duration" 'BEGIN { exit !(d >= 20 && d <= 60) }' ||
      die "$name.mp4 lasts $duration s (expected 20 to 60)"
    ffmpeg -v error -i "$src" -vf "fps=$GIF_FPS,scale=320:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff:max_colors=64[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
      -loop 0 -y "$tmp/$name.gif"
    gif_frames=$(stream "$tmp/$name.gif" nb_read_frames)
    [ "$gif_frames" -gt 1 ] || die "$name.gif is not animated"
    check "$tmp/$name.gif" "gif,320,$gif_frames" codec_name,width,nb_read_frames
    mv -f "$tmp/$name.gif" "$OUT/$name.gif"
  fi
  mv -f "$tmp/$name.mp4" "$OUT/$name.mp4"
  echo "encode.sh: wrote $OUT/$name.mp4: $frames frames at $FPS fps, $duration s ($ENCODER)${gif:+, GIF $gif_frames frames}" >&2
}

case "${1:-final}" in
  final) clip final-tour "$FINAL_SECONDS" "${NAME:-01-final-tour}" gif ;;
  tour) clip tour "$TOUR_SECONDS" "${NAME:-00-provisional-tour}" ;;
  *) die "usage: $0 [final|tour]" ;;
esac

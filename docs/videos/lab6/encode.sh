#!/usr/bin/env bash
# Regenerates the laboratory 6 evidence videos (MP4 + GIF) from the renderer.
#
#   docs/videos/lab6/encode.sh all
#   docs/videos/lab6/encode.sh single <name> <seconds> -- <renderer args...>
#   docs/videos/lab6/encode.sh pair <name> <seconds> "<args A>" "<args B>" "<label A>" "<label B>"
#
# Run from the repository root after `cargo build --release`. Frames come
# from `--record` (raw rgb24 at 800x600) and go through a lossless FFV1
# intermediate; the GIF is made from that intermediate and never depends on
# H.264. The H.264 encoder is chosen by a functional test (encode one second,
# check the codec, decode it) in this order: libx264, h264_nvenc,
# h264_vaapi, h264_qsv, libopenh264. H264_ENCODER (and VAAPI_DEVICE) may be
# given in the environment; they are validated the same way. Without any
# working encoder the script stops, unless ALLOW_GIF_ONLY=1 (GIF only).
set -euo pipefail

BIN="${BIN:-./target/release/rust-software-renderer}"
OUT="${OUT:-docs/videos/lab6}"
FPS=30
GIF_FPS="${GIF_FPS:-8}"
# Defaults keep every MP4 and GIF under about 4 MB (noisy procedural content).
QP="${QP:-32}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

die() { echo "encode.sh: $*" >&2; exit 1; }
[ -x "$BIN" ] || die "renderer not found at $BIN (run cargo build --release)"
mkdir -p "$OUT"

# With pipefail, "ffmpeg -filters | grep -q" can fail by SIGPIPE, so the
# filter list is captured once.
FILTERS="$(ffmpeg -hide_banner -filters 2>/dev/null)"
has_filter() { grep -q " $1 " <<<"$FILTERS"; }
for f in hstack palettegen paletteuse; do has_filter "$f" || die "ffmpeg lacks the $f filter"; done
FONT=""
if has_filter drawtext && command -v fc-match >/dev/null; then
  FONT="$(fc-match -f '%{file}' 'Adwaita Sans' 2>/dev/null || true)"
fi

pre_input=()
encoder_options() { # options for the chosen encoder
  case "$1" in
    libx264) echo "-c:v libx264 -preset medium -crf ${CRF:-28} -pix_fmt yuv420p" ;;
    # Constant QP without B-frames or adaptive quantization: avoids the grey
    # blocks and ghost lines observed with NVENC in laboratory 5.
    h264_nvenc) echo "-c:v h264_nvenc -rc constqp -qp $QP -bf 0 -spatial-aq 0 -temporal-aq 0 -pix_fmt yuv420p" ;;
    h264_vaapi) echo "-vf format=nv12,hwupload -c:v h264_vaapi -qp $QP" ;;
    h264_qsv) echo "-c:v h264_qsv -global_quality $QP -pix_fmt nv12" ;;
    libopenh264) echo "-c:v libopenh264 -b:v ${BITRATE:-700k} -pix_fmt yuv420p" ;;
    *) return 1 ;;
  esac
}
pre_for() { if [ "$1" = h264_vaapi ]; then echo "-vaapi_device ${2:-/dev/dri/renderD128}"; fi; }

probe() { # probe <encoder> [vaapi device]: encode 1 s, check h264, decode
  local enc="$1" dev="${2:-}" out="$tmp/probe-$1.mp4"
  # shellcheck disable=SC2046
  ffmpeg -hide_banner -v error $(pre_for "$enc" "$dev") -f lavfi -i testsrc2=size=320x240:rate=30 -t 1 \
    $(encoder_options "$enc") -y "$out" 2>/dev/null &&
    [ "$(ffprobe -v error -select_streams v:0 -show_entries stream=codec_name -of csv=p=0 "$out")" = h264 ] &&
    ffmpeg -v error -i "$out" -f null - 2>/dev/null
}

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

# record <seconds> <file.mkv> <renderer args...>: lossless 30 fps intermediate.
record() {
  local seconds="$1" file="$2"; shift 2
  # The raw stream has no timing: declare the rate before -i or the rawvideo
  # demuxer assumes 25 fps and stretches the video.
  "$BIN" --record "$seconds" --fps "$FPS" "$@" 2>/dev/null |
    ffmpeg -v error -f rawvideo -pix_fmt rgb24 -s 800x600 -framerate "$FPS" -i - \
      -c:v ffv1 -level 3 -y "$file"
  local want frames rate
  want=$(awk -v s="$seconds" -v f="$FPS" 'BEGIN { x = s * f; print (x == int(x)) ? x : int(x) + 1 }')
  frames=$(ffprobe -v error -count_frames -select_streams v:0 -show_entries stream=nb_read_frames -of csv=p=0 "$file")
  rate=$(ffprobe -v error -select_streams v:0 -show_entries stream=r_frame_rate -of csv=p=0 "$file")
  [ "$frames" = "$want" ] && [ "$rate" = "$FPS/1" ] ||
    die "intermediate $file has $frames frames at $rate (expected $want at $FPS/1)"
  echo "encode.sh: intermediate $(basename "$file"): $frames frames at $rate" >&2
}

# finish <intermediate> <name> <gif width>: MP4 (if an encoder works) and GIF.
finish() {
  local src="$1" name="$2" gif_width="$3"
  if [ -n "$ENCODER" ]; then
    # shellcheck disable=SC2046
    ffmpeg -v error $(pre_for "$ENCODER" "${VAAPI_DEVICE:-}") -i "$src" $(encoder_options "$ENCODER") \
      -r "$FPS" -movflags +faststart -y "$OUT/$name.mp4"
  fi
  ffmpeg -v error -i "$src" -vf "fps=$GIF_FPS,scale=$gif_width:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff:max_colors=64[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
    -loop 0 -y "$OUT/$name.gif"
  echo "encode.sh: wrote $name (${ENCODER:-gif only})" >&2
}

label() { # drawtext filter for a panel label, or a no-op without drawtext
  if [ -n "$FONT" ]; then
    echo "drawtext=fontfile=$FONT:text='$1':x=20:y=20:fontsize=30:fontcolor=white:box=1:boxcolor=black@0.55:boxborderw=8"
  else
    echo "null"
  fi
}

single() { # single <name> <seconds> -- <args...>
  local name="$1" seconds="$2"; shift 2; [ "${1:-}" = -- ] && shift
  record "$seconds" "$tmp/$name.mkv" "$@"
  finish "$tmp/$name.mkv" "$name" 320
}

pair() { # pair <name> <seconds> "<args A>" "<args B>" "<label A>" "<label B>"
  local name="$1" seconds="$2" a="$3" b="$4" la="$5" lb="$6"
  # shellcheck disable=SC2086
  record "$seconds" "$tmp/$name-a.mkv" $a
  # shellcheck disable=SC2086
  record "$seconds" "$tmp/$name-b.mkv" $b
  ffmpeg -v error -i "$tmp/$name-a.mkv" -i "$tmp/$name-b.mkv" \
    -filter_complex "[0]$(label "$la")[l];[1]$(label "$lb")[r];[l][r]hstack=inputs=2" \
    -c:v ffv1 -level 3 -y "$tmp/$name.mkv"
  finish "$tmp/$name.mkv" "$name" 640
}

all() {
  local entrance="--scene entrance"
  single 01-kings-crown-material 24 -- --scene crown --timeline material
  single 02-crimson-entrance-events 32 -- $entrance --timeline material
  single 03-house-of-cards-vortex 36 -- --scene house --timeline material
  pair 04-entrance-cloud-shadows-light 12 \
    "$entrance --timeline light --time 8 --freeze-time --set cloud_shadows=0" \
    "$entrance --timeline light --time 8 --freeze-time --set cloud_shadows=1" \
    "cloud_shadows = 0" "cloud_shadows = 1"
  # Effects that do not involve the clouds are recorded without them, and
  # parallax at a demonstration scale, so each clip isolates one effect.
  pair 05-entrance-bump-light 12 \
    "$entrance --timeline light --time 8 --freeze-time --light-elevation 8 --set clouds=0 --set bump=0" \
    "$entrance --timeline light --time 8 --freeze-time --light-elevation 8 --set clouds=0 --set bump=1" \
    "bump = 0" "bump = 1"
  # The orbit changes every pixel of every frame: a higher QP keeps it < 4 MB.
  QP=34 pair 06-entrance-parallax-orbit 12 \
    "$entrance --timeline orbit --time 8 --freeze-time --set clouds=0 --set parallax_scale=0.12 --set parallax=0" \
    "$entrance --timeline orbit --time 8 --freeze-time --set clouds=0 --set parallax_scale=0.12 --set parallax=1" \
    "parallax = 0" "parallax = 1 (scale 0.12)"
  single 07-entrance-day-night-light 12 -- $entrance --timeline light --time 8 --freeze-time
  single 08-entrance-lighting-responses 12 -- $entrance --timeline light --time 8 --freeze-time --light-elevation 5 --set clouds=0
  pair 09-entrance-clouds-drift 16 \
    "$entrance --timeline material --set events=0" \
    "$entrance --timeline material --set events=0 --debug-view clouds" \
    "imagen final" "debug-view clouds"
}

case "${1:-}" in
  all) all ;;
  single) shift; single "$@" ;;
  pair) shift; pair "$@" ;;
  *) die "usage: encode.sh all | single <name> <seconds> -- <args> | pair <name> <seconds> <args A> <args B> <label A> <label B>" ;;
esac

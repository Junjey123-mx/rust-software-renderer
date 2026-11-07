#!/usr/bin/env bash
# Regenerates the laboratory 6 evidence (MP4 + GIF videos and the comparison
# montages) from the renderer.
#
#   docs/videos/lab6/encode.sh all      every video (01-10)
#   docs/videos/lab6/encode.sh stills   the comparison montages 05-09 (PNG)
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
# Every file is written to a temporary directory and checked (codec, size,
# frame count, full decode) before it replaces the previous evidence in OUT
# (videos) or SHOTS (montages).
set -euo pipefail

BIN="${BIN:-./target/release/rust-software-renderer}"
OUT="${OUT:-docs/videos/lab6}"
SHOTS="${SHOTS:-docs/screenshots/lab6}"
FPS=30
GIF_FPS="${GIF_FPS:-8}"
# Defaults keep every MP4 and GIF under about 4 MB (noisy procedural content).
QP="${QP:-32}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

die() { echo "encode.sh: $*" >&2; exit 1; }
[ -x "$BIN" ] || die "renderer not found at $BIN (run cargo build --release)"
mkdir -p "$OUT" "$SHOTS"

# stream <file> <entries>: one ffprobe line for the first video stream.
stream() {
  ffprobe -v error -count_frames -select_streams v:0 -show_entries "stream=$2" -of csv=p=0 "$1"
}
# check <file> <expected "codec,width,height[,...]"> <entries>: the file is
# not empty, has the expected stream and decodes without errors.
check() {
  local file="$1" want="$2" got
  [ -s "$file" ] || die "$file is empty"
  got=$(stream "$file" "$3")
  [ "$got" = "$want" ] || die "$file has $got (expected $want)"
  [ -z "$(ffmpeg -v error -i "$file" -f null - 2>&1)" ] || die "$file does not decode cleanly"
}

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
# Both are checked before they replace the previous files.
finish() {
  local src="$1" name="$2" gif_width="$3" size frames gif_frames
  size=$(stream "$src" width,height)
  frames=$(stream "$src" nb_read_frames)
  if [ -n "$ENCODER" ]; then
    # shellcheck disable=SC2046
    ffmpeg -v error $(pre_for "$ENCODER" "${VAAPI_DEVICE:-}") -i "$src" $(encoder_options "$ENCODER") \
      -r "$FPS" -movflags +faststart -y "$tmp/$name.mp4"
    check "$tmp/$name.mp4" "h264,$size,$FPS/1,$frames" codec_name,width,height,r_frame_rate,nb_read_frames
  fi
  ffmpeg -v error -i "$src" -vf "fps=$GIF_FPS,scale=$gif_width:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff:max_colors=64[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
    -loop 0 -y "$tmp/$name.gif"
  gif_frames=$(stream "$tmp/$name.gif" nb_read_frames)
  [ "$gif_frames" -gt 1 ] || die "$name.gif is not animated"
  check "$tmp/$name.gif" "gif,$gif_width,$gif_frames" codec_name,width,nb_read_frames
  if [ -n "$ENCODER" ]; then mv -f "$tmp/$name.mp4" "$OUT/$name.mp4"; fi
  mv -f "$tmp/$name.gif" "$OUT/$name.gif"
  echo "encode.sh: wrote $name: $frames frames, GIF $gif_frames frames (${ENCODER:-gif only})" >&2
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
  # Fixed camera (E01): the three front loop regions flare in turn, then the eclipse.
  single 01-kings-crown-material 24 -- --scene crown --timeline material
  # From t = 27 s the default view sees three spade craters of different
  # sizes open in turn, two of them at once in 35-40 s and 42-47 s.
  single 02-crimson-entrance-events 30 -- $entrance --timeline material --time 27
  # A quarter turn from the side view shows the full cycle of the medium
  # (6-22 s), small (16-28 s) and large (22-34 s) storms.
  single 03-house-of-cards-vortex 36 -- --scene house --view side --timeline showcase
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
  # Same simulated time from two fixed cameras: the oblique view sees the
  # even loop slots and the side view the odd ones, on the other hemisphere.
  # Two panels of noisy plasma: QP 34 and a 5 fps GIF keep each file near 4 MB.
  QP=34 GIF_FPS=5 pair 10-kings-crown-hemispheres 24 \
    "--scene crown --timeline material" \
    "--scene crown --timeline material --view side" \
    "vista oblicua" "vista lateral"
}

# frame <file.png> <renderer args...>: the first frame of a recording.
frame() {
  local file="$1"; shift
  "$BIN" --record 0.04 --fps 25 "$@" 2>/dev/null |
    ffmpeg -v error -f rawvideo -pix_fmt rgb24 -s 800x600 -framerate 25 -i - -frames:v 1 -y "$file"
}

# montage <name> <panel width> "<args>" "<label>" ...: labelled panels side by side.
montage() {
  local name="$1" width="$2" n=0 inputs=() filters="" pads="" height
  shift 2
  while [ $# -gt 0 ]; do
    # shellcheck disable=SC2086
    frame "$tmp/$name-$n.png" $1
    inputs+=(-i "$tmp/$name-$n.png")
    filters+="[$n]$(label "$2"),scale=$width:-2:flags=lanczos[p$n];"
    pads+="[p$n]"
    n=$((n + 1))
    shift 2
  done
  ffmpeg -v error "${inputs[@]}" -filter_complex "${filters}${pads}hstack=inputs=$n" -frames:v 1 -y "$tmp/$name.png"
  height=$((width * 600 / 800))
  check "$tmp/$name.png" "png,$((width * n)),$height" codec_name,width,height
  mv -f "$tmp/$name.png" "$SHOTS/$name.png"
  echo "encode.sh: wrote $SHOTS/$name.png" >&2
}

# The montages share t = 8 s, when the large spade crater glows in front of
# the default camera, and t = 30 s for House of Cards (large and medium storms).
stills() {
  local entrance="--scene entrance --time 8"
  montage 05-entrance-bump-off-on 800 \
    "$entrance --light-elevation 8 --set clouds=0 --set bump=0" "bump = 0" \
    "$entrance --light-elevation 8 --set clouds=0 --set bump=1" "bump = 1"
  montage 06-entrance-cloud-coverage 600 \
    "$entrance --set cloud_coverage=0.2" "cloud_coverage = 0.2" \
    "$entrance --set cloud_coverage=0.45" "cloud_coverage = 0.45" \
    "$entrance --set cloud_coverage=0.7" "cloud_coverage = 0.7"
  montage 07-entrance-cloud-shadow-masks 600 \
    "$entrance" "imagen final" \
    "$entrance --debug-view clouds" "debug-view clouds" \
    "$entrance --debug-view shadows" "debug-view shadows"
  montage 08-entrance-materials 800 \
    "$entrance --set clouds=0" "imagen final (clouds = 0)" \
    "$entrance --set clouds=0 --debug-view materials" "debug-view materials"
  montage 09-house-vortex-off-on 800 \
    "--scene house --time 30 --set lightning=0 --set vortex=0" "vortex = 0" \
    "--scene house --time 30 --set lightning=0 --set vortex=1" "vortex = 1"
}

case "${1:-}" in
  all) all ;;
  stills) stills ;;
  single) shift; single "$@" ;;
  pair) shift; pair "$@" ;;
  *) die "usage: encode.sh all | stills | single <name> <seconds> -- <args> | pair <name> <seconds> <args A> <args B> <label A> <label B>" ;;
esac

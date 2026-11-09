#!/usr/bin/env bash
# The diver's clips, from Long Time Comin's copy of Style Lab's clip script:
# author and bake every clip on the costumed diver (tools/mesh/src/diver.blend)
# into tools/mesh/src/ltc.blend and ltc_clips.json, then bake the diver for the
# browser (tools/mesh/bake_diver.py). No videos (--no-render).
#   tools/mesh/make_clips.sh [--fast]
set -euo pipefail
cd "$(dirname "$0")/../.."
B="${BLENDER:-$(command -v blender || echo /Applications/Blender.app/Contents/MacOS/Blender)}"
"$B" -b tools/mesh/src/diver.blend --python tools/mesh/clips/ltc_clips.py -- tools/mesh/src --no-render 2>&1 | grep -E "^ltc_clips:|Error|Traceback" || true
"$B" -b tools/mesh/src/ltc.blend --python tools/mesh/bake_diver.py -- "$@" 2>&1 | grep -E "^bake_diver: [0-9]+ bones|Error:|Traceback" | grep -v "Modifier is disabled" || true
echo "make_clips: done"

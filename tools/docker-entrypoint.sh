#!/bin/sh
# Container entry for arda-server. With no arguments it serves the world
# mounted at /worlds/world on 0.0.0.0:8787 with the best library the image
# holds (the AI library stacked over the placeholder when present, else the
# placeholder alone). ARDA_LIBRARY overrides the library, ARDA_WORLD the
# world path. With arguments, they go to arda-server verbatim.
set -eu

if [ "$#" -gt 0 ]; then
    exec arda-server "$@"
fi

lib="${ARDA_LIBRARY:-}"
if [ -z "$lib" ]; then
    if [ -f /opt/arda/library/ai/catalog.json ]; then
        lib=/opt/arda/library/ai:/opt/arda/library/placeholder
    else
        lib=/opt/arda/library/placeholder
    fi
fi

exec arda-server --world "${ARDA_WORLD:-/worlds/world}" --host 0.0.0.0 \
    --port 8787 --library "$lib"

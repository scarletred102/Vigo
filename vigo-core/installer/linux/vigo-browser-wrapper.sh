#!/bin/bash
# Copyright (c) 2025 Vigo Browser. All rights reserved.
# Proprietary and confidential. Unauthorized copying prohibited.
#
# Wrapper script for Vigo Browser on Linux.
# Handles library paths and environment setup.

VIGO_DIR="/usr/lib/vigo-browser"
if [ -d "/usr/lib64/vigo-browser" ]; then
    VIGO_DIR="/usr/lib64/vigo-browser"
fi

# Add Vigo libraries to LD_LIBRARY_PATH
export LD_LIBRARY_PATH="${VIGO_DIR}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

# Enable Wayland if available and not overridden
if [ -z "$VIGO_DISABLE_WAYLAND" ] && [ "$XDG_SESSION_TYPE" = "wayland" ]; then
    WAYLAND_FLAGS="--ozone-platform=wayland --enable-features=WaylandWindowDecorations"
fi

# Enable VAAPI hardware decode
VA_FLAGS="--enable-features=VaapiVideoDecoder,VaapiVideoEncoder"

# Enable GPU acceleration
GPU_FLAGS="--enable-gpu-rasterization --enable-zero-copy"

# Launch Vigo
exec "${VIGO_DIR}/vigo-browser" \
    ${WAYLAND_FLAGS:-} \
    ${VA_FLAGS} \
    ${GPU_FLAGS} \
    "$@"

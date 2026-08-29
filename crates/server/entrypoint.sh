#!/bin/sh
set -e

# The attachments dir is usually a freshly-mounted volume, which Docker
# creates owned by root regardless of what USER the image declares - so this
# has to run as root once, then hand off to the unprivileged user before
# ever touching the network or an uploaded file.
mkdir -p "$ATTACHMENTS_DIR"
chown -R hollowchat:hollowchat "$ATTACHMENTS_DIR"

exec gosu hollowchat /usr/local/bin/hollowchat-server

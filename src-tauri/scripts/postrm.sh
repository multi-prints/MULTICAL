#!/bin/bash
# Post-remove script for MULTIPRINTS (.deb)
# Removes app data (database, config) left behind after package removal
set -e

APP_ID="com.multiprints.desktop"

# Remove per-user app data directories
for dir in \
  "${HOME}/.local/share/${APP_ID}" \
  "${HOME}/.config/${APP_ID}" \
  "${HOME}/.cache/${APP_ID}"; do
  if [ -d "$dir" ]; then
    rm -rf "$dir"
  fi
done

echo "MULTIPRINTS app data removed."

exit 0

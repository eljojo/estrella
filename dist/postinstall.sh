#!/bin/bash
set -e

# Create estrella system user if it doesn't exist
if ! id -u estrella >/dev/null 2>&1; then
    useradd --system --no-create-home --shell /usr/sbin/nologin estrella
fi

# Ensure estrella user is in dialout group (for /dev/rfcomm access)
usermod -aG dialout estrella 2>/dev/null || true

# Reload systemd to pick up new unit files
systemctl daemon-reload

echo ""
echo "========================================="
echo "  estrella installed successfully!"
echo "========================================="
echo ""
echo "Next steps:"
echo "  1. Edit /etc/estrella/estrella.conf"
echo "     Set DEVICE_MAC to your printer's Bluetooth MAC address"
echo ""
echo "  2. Pair your printer (if not already paired):"
echo "     sudo bluetoothctl"
echo "     > power on"
echo "     > agent on"
echo "     > scan on"
echo "     > pair XX:XX:XX:XX:XX:XX"
echo "     > trust XX:XX:XX:XX:XX:XX"
echo ""
echo "  3. Enable and start the services:"
echo "     sudo systemctl enable --now estrella-rfcomm estrella"
echo ""
echo "  4. Open http://localhost:8080 in your browser"
echo ""
echo "IMPORTANT: Your Star printer must have SSP mode disabled"
echo "via the Star Settings iOS/Android app. See the README for details."
echo ""

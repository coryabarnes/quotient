#!/bin/sh
set -eu
cd "$(dirname "$0")"
prefix="${HOME}/.local"
binary="target/release/quotient"
if [ -f ./quotient ]; then
    binary="./quotient"
fi
if [ ! -x "$binary" ]; then
    echo "Build Quotient first with: cargo build --release" >&2
    exit 1
fi
install -Dm755 "$binary" "$prefix/bin/quotient.new"
mv "$prefix/bin/quotient.new" "$prefix/bin/quotient"
install -Dm644 assets/quotient.svg "$prefix/share/icons/hicolor/scalable/apps/quotient.svg"
install -Dm644 assets/quotient.png "$prefix/share/icons/hicolor/1024x1024/apps/quotient.png"
install -Dm644 assets/quotient-256.png "$prefix/share/icons/hicolor/256x256/apps/quotient.png"
mkdir -p "$prefix/share/applications"
cat > "$prefix/share/applications/quotient.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Quotient
GenericName=Calculator
Comment=Math in plain text, with units, complex numbers and high precision
Exec="$prefix/bin/quotient" %f
Icon=$prefix/share/icons/hicolor/256x256/apps/quotient.png
Terminal=false
Categories=Utility;Calculator;
Keywords=calculator;math;units;convert;notepad;calctxt;
StartupWMClass=quotient
EOF
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$prefix/share/applications"
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$prefix/share/icons/hicolor"
fi
if command -v kbuildsycoca6 >/dev/null 2>&1; then
    kbuildsycoca6 --noincremental
fi
echo "Quotient installed. Open it from your application launcher."

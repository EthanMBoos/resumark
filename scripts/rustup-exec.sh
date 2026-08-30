#!/bin/sh
set -eu

unset NO_COLOR

if command -v brew >/dev/null 2>&1 \
  && rustup_prefix="$(brew --prefix rustup 2>/dev/null)" \
  && [ -x "$rustup_prefix/bin/cargo" ]; then
  rustup_bin="$rustup_prefix/bin"
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
  rustup_bin="$HOME/.cargo/bin"
else
  echo "Could not find the rustup command proxies." >&2
  echo "Install rustup, then try this command again." >&2
  exit 1
fi

PATH="$rustup_bin:$PATH"
export PATH

exec "$@"

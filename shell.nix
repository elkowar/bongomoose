let
  moz_overlay = import (builtins.fetchTarball
    https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
in with pkgs;
mkShell {
  buildInputs = [
    latest.rustChannels.nightly.rust
    # (rustChannelOf { date = "2020-05-28"; channel = "nightly"; }).rust
  ];
}

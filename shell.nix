{ pkgs ? import <nixpkgs> {} }:

with pkgs; stdenv.mkDerivation rec {
  name = "arborio-env";
  nativeBuildInputs = [
    rustup
  ];
  buildInputs = [
    gnome.zenity
    xlibs.libxcb
  ];
  LD_LIBRARY_PATH = "${lib.makeLibraryPath [
    libGL
    xlibs.libX11
    xlibs.libXcursor
    xlibs.libxcb
    xlibs.libXi
    xlibs.libXrandr
  ]}";
}


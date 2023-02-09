{
  inputs = rec {
    nixpkgs.url = "github:NixOs/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    flake-compat = {
      url = github:edolstra/flake-compat;
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, naersk, flake-compat }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
        };

        naersk-lib = pkgs.callPackage naersk {};
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
          libGL
          xorg.libX11
          xorg.libXcursor
          xorg.libxcb
          xorg.libXi
          xorg.libXrandr
        ]);
        buildInputs = with pkgs; [ gnome.zenity xorg.libxcb pkg-config fontconfig ];
        pname = "arborio";
      in
        rec {
          # `nix build`
          packages.arborio = naersk-lib.buildPackage {
            inherit pname;
            gitAllRefs = true;
            root = ./.;
            buildInputs = buildInputs ++ [ pkgs.cmake pkgs.makeWrapper ];
            #nativeBuildInputs = [ pkgs.breakpointHook ];
            overrideMain = (self: self // {
              postFixup = self.postFixup or '''' + ''
                wrapProgram $out/bin/arborio --set LD_LIBRARY_PATH "${LD_LIBRARY_PATH}"
              '';
            });
            cargoBuildOptions = x: x ++ ["--package" "${pname}"];
          };
          packages.default = packages.arborio;

          # `nix run`
          apps.arborio = flake-utils.lib.mkApp {
            drv = packages.arborio;
          };
          apps.default = apps.arborio;

          # `nix develop`
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [ cmake pkg-config ];
            inherit buildInputs LD_LIBRARY_PATH;
          };
        }
    );
}

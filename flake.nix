{
  inputs = rec {
    nixpkgs.url = "github:NixOs/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    mozillapkgs = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };
    flake-compat = {
      url = github:edolstra/flake-compat;
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, naersk, mozillapkgs, flake-compat }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (self: super: {
              cargo = rust;
              rustc = rust;
            })
          ];
        };

        mozilla = pkgs.callPackage (mozillapkgs + "/package-set.nix") {};
        rust = (mozilla.rustChannelOf {
          channel = "stable";
          version = "1.60.0";
          sha256 = "sha256-otgm+7nEl94JG/B+TYhWseZsHV1voGcBsW/lOD2/68g=";
        }).rust;

        naersk-lib = pkgs.callPackage naersk {};
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
          libGL
          xorg.libX11
          xorg.libXcursor
          xorg.libxcb
          xorg.libXi
          xorg.libXrandr
        ]);
        buildInputs = with pkgs; [ gnome.zenity xorg.libxcb ];
        #naersk-lib = naersk.lib."${system}".override {
        #  cargo = rust;
        #  rustc = rust;
        #};
      in
        rec {
          # `nix build`
          packages.arborio = naersk-lib.buildPackage {
            pname = "arborio";
            gitAllRefs = true;
            root = ./.;
            buildInputs = buildInputs ++ [ pkgs.makeWrapper ];
            overrideMain = (self: self // {
              postFixup = self.postFixup or '''' + ''
                wrapProgram $out/bin/arborio --set LD_LIBRARY_PATH "${LD_LIBRARY_PATH}"
              '';
            });
          };
          defaultPackage = packages.arborio;

          # `nix run`
          apps.arborio = flake-utils.lib.mkApp {
            drv = packages.arborio;
          };
          defaultApp = apps.arborio;

          # `nix develop`
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [ rustc cargo ];
            inherit buildInputs LD_LIBRARY_PATH;
          };
        }
    );
}

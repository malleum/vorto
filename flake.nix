{
  description = "Vorto - TUI Bible Viewer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Build the sqlite db using our python script and local zips
        bibles_db = pkgs.stdenv.mkDerivation {
          name = "vorto-bibles-db";
          src = ./.;
          nativeBuildInputs = [ pkgs.python3 pkgs.sqlite ];
          buildPhase = ''
            python3 build_db.py bibles.db data/engbsb_usfm.zip data/engwebp_usfm.zip data/englsv_usfm.zip data/epo_usfm.zip data/latVUC_usfm.zip data/eng-kjv2006_usfm.zip data/noblb_usfm.zip
          '';
          installPhase = ''
            mkdir -p $out/share/vorto
            cp bibles.db $out/share/vorto/
          '';
        };

      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "vorto";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          buildInputs = [ pkgs.sqlite ];
          nativeBuildInputs = [ pkgs.makeWrapper ];
          postInstall = ''
            wrapProgram $out/bin/vorto \
              --set VORTO_DB_PATH ${bibles_db}/share/vorto/bibles.db \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.wl-clipboard pkgs.xclip ]}
          '';
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            sqlite
            python3
            wl-clipboard
            xclip
          ];
          shellHook = ''
            export VORTO_DB_PATH="${bibles_db}/share/vorto/bibles.db"
          '';
        };
      }
    );
}

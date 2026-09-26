{
  description = "Intuitive Bible reader";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "charist";
          version = "0.3.3";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkg-config ];

          buildInputs = [
            pkgs.wayland
            pkgs.libxkbcommon
          ];

          # Matches PKGBUILD's options=('!strip' '!lto')
          dontStrip = true;

          postInstall = ''
            install -Dm644 resources/org.barbel.Charist.desktop \
              "$out/share/applications/org.barbel.Charist.desktop"
            install -Dm644 resources/icons/hicolor/scalable/apps/org.barbel.Charist.svg \
              "$out/share/icons/hicolor/scalable/apps/org.barbel.Charist.svg"
          '';

          meta = with pkgs.lib; {
            description = "Intuitive Bible reader";
            license = licenses.agpl3Only;
            platforms = [ "x86_64-linux" ];
            mainProgram = "charist";
          };
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          nativeBuildInputs = [ pkgs.cargo pkgs.rustc pkgs.lld ];
        };
      });
}
{
  description = "Intuitive Bible reader";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.mkLib pkgs;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.lld
          ];
          buildInputs = [
            pkgs.wayland
            pkgs.libxkbcommon
          ];
        };

        # Build deps once, separately, so rebuilds of just your code are fast
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        charist = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "charist";
          version = "0.3.3";

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
        });
      in
      {
        packages.default = charist;

        devShells.default = craneLib.devShell {
          inputsFrom = [ charist ];
          packages = [ pkgs.lld ];
        };
      });
}
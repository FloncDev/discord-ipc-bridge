{
  description = "beehive development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };

        runtimeDeps = with pkgs; [
          openssl pkg-config libxkbcommon libGL
          libX11 libXcursor libXi libXrandr
        ];
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain

            # For flake editing
            pkgs.nil
            pkgs.nixfmt
            pkgs.nixd
          ] ++ runtimeDeps;

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath runtimeDeps}:$LD_LIBRARY_PATH
            export PKG_CONFIG_PATH=${pkgs.openssl.dev}/lib/pkgconfig
          '';
        };
      }
    );
}

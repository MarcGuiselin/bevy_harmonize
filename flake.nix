{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, fenix }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          craneLib = (crane.mkLib pkgs).overrideToolchain (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
            "rust-src"
            "rustfmt"
          ]);
          buildInputs = with pkgs; [
            # Dev tools
            nixd
            
            # Build tools
            pkg-config
          ] ++ lib.optionals stdenv.isLinux [
            alsa-lib
            libxkbcommon
            udev
            vulkan-loader
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk_11_0.frameworks.Cocoa
            rustPlatform
          ];
        in
        {
          devShells.default = craneLib.devShell {
            inherit buildInputs;

            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
            RUSTFLAGS="-Z crate-attr=feature(const_trait_impl)";
          };
        }
      );
}
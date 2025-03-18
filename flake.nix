{
  inputs = {
    fenix = {
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-analyzer-src = {
          flake = false;
          url = "github:rust-lang/rust-analyzer/nightly";
        };
      };
      url = "github:nix-community/fenix";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    treefmt-nix = {
      inputs.nixpkgs.follows = "nixpkgs";
      url = "github:numtide/treefmt-nix";
    };
  };
  outputs =
    {
      fenix,
      flake-utils,
      nixpkgs,
      self,
      treefmt-nix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        toolchain =
          with fenix.packages.${system};
          combine [
            complete.toolchain
            targets.${(builtins.fromTOML (builtins.readFile ./rp/.cargo/config.toml)).build.target}.latest.rust-std
          ];
        pkgs = import nixpkgs { inherit system; };
        treefmt = treefmt-nix.lib.evalModule pkgs ./.treefmt.nix;
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            [ toolchain ]
            ++ (with pkgs; [
              cargo-expand
              flip-link
              picotool
              probe-rs-tools
            ]);
          # DEFMT_LOG = "debug";
          # RUST_BACKTRACE = "1";
        };
        formatter = treefmt.config.build.wrapper;
      }
    );
}

{
  inputs = {
    # fenix = {
    #   inputs = {
    #     nixpkgs.follows = "nixpkgs";
    #     rust-analyzer-src = {
    #       flake = false;
    #       url = "github:rust-lang/rust-analyzer/nightly";
    #     };
    #   };
    #   url = "github:nix-community/fenix";
    # };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      inputs.nixpkgs.follows = "nixpkgs";
      url = "github:numtide/treefmt-nix";
    };
  };
  outputs =
    {
      # fenix,
      flake-utils,
      nixpkgs,
      rust-overlay,
      self,
      treefmt-nix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let

        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        # rust =
        #   with fenix.packages.${system};
        #   combine [
        #     complete.toolchain
        #     targets.${(builtins.fromTOML (builtins.readFile ./rp/.cargo/config.toml)).build.target}.latest.rust-std
        #   ];
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        treefmt = treefmt-nix.lib.evalModule pkgs ./.treefmt.nix;

      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            [
              rust
            ]
            ++ (with pkgs; [
              cargo-expand
              flip-link
              openssl
              picotool
              pkg-config
              probe-rs-tools
            ]);
          # DEFMT_LOG = "debug";
          # RUST_BACKTRACE = "1";
        };

        formatter = treefmt.config.build.wrapper;
      }
    );
}

{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs.buildPackages; [ cargo rustc clippy bacon rust-analyzer 
    pkg-config
    openssl
    alsaLib
  ];
}

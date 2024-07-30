{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    pcsclite pkg-config
  ];

  LD_LIBRARY_PATH = "${pkgs.pcsclite.lib}/lib/";
}

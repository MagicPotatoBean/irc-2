with import <nixpkgs> { };

stdenv.mkDerivation {
  name = "mydevshell";

  buildInputs = with pkgs; [
    ncurses
  ];
  shellHook = ''
    export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [pkgs.ncurses]}:$LD_LIBRARY_PATH;
  '';
}


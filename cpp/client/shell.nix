{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # C++ compiler
    gcc

    # Build tools
    pkg-config
    gnumake

    # Protocol buffers (v21 for C++14 compatibility)
    protobuf_21

    # SDL2 for windowing and input
    SDL2

    # OpenGL
    libGL
    mesa
  ];

  shellHook = ''
    echo "Earth Reverse Engineering Client - Development Shell"
    echo "Build with: ./build.sh"
    echo "Run with: ./main"

    # Wrap g++ and c++ to add necessary flags for this codebase
    # -Wno-changes-meaning: allow variables with same name as their type
    mkdir -p .nix-wrappers
    REAL_GXX=$(which g++)
    REAL_CXX=$(which c++)
    echo "#!/bin/sh" > .nix-wrappers/g++
    echo "exec $REAL_GXX -Wno-changes-meaning \"\$@\"" >> .nix-wrappers/g++
    echo "#!/bin/sh" > .nix-wrappers/c++
    echo "exec $REAL_CXX -Wno-changes-meaning \"\$@\"" >> .nix-wrappers/c++
    chmod +x .nix-wrappers/g++ .nix-wrappers/c++
    export PATH="$PWD/.nix-wrappers:$PATH"
  '';
}

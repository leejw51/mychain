with import <nixpkgs> {};

let
  pythonEnv = python37.withPackages (ps: [  
    ps.pip
    ps.docker
    ps.setuptools
    ps.wheel
    ps.jsonpatch
    ps.fire
    ps.toml
    ps.pynacl
    ps.mnemonic
]);

in mkShell {
  buildInputs = [
    pythonEnv
    hello
  ];
}
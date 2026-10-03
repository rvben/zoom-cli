{
  lib,
  stdenv,
  rustPlatform,
  installShellFiles,
}:

let
  manifest = lib.importTOML ../Cargo.toml;
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = manifest.package.name;
  version = manifest.package.version;
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../tests
      ../README.md
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ installShellFiles ];
  doCheck = stdenv.buildPlatform.canExecute stdenv.hostPlatform;
  # Tests exercise HTTP mock servers on loopback.
  __darwinAllowLocalNetworking = true;

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    for shell in bash fish zsh; do
      "$out/bin/zoom" completions "$shell" > "zoom.$shell"
    done
    installShellCompletion zoom.{bash,fish,zsh}
  '';

  doInstallCheck = stdenv.buildPlatform.canExecute stdenv.hostPlatform;
  installCheckPhase = ''
    runHook preInstallCheck
    test "$("$out/bin/zoom" --version)" = "zoom ${finalAttrs.version}"
    "$out/bin/zoom" --help > /dev/null
    for completion in \
      "$out/share/bash-completion/completions/zoom.bash" \
      "$out/share/fish/vendor_completions.d/zoom.fish" \
      "$out/share/zsh/site-functions/_zoom"; do
      if ! test -s "$completion"; then
        echo "Missing or empty completion file: $completion" >&2
        exit 1
      fi
    done
    runHook postInstallCheck
  '';

  meta = {
    inherit (manifest.package) description homepage;
    license = lib.licenses.mit;
    mainProgram = "zoom";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ];
  };
})

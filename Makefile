CARGO_DIRS := $(patsubst %Cargo.toml,task/%,$(shell git ls-tree -r HEAD --name-only | grep Cargo.toml))
ALL_TARGETS := \
  $(patsubst %,%build.release,$(CARGO_DIRS)) \
  $(patsubst %,%check.release,$(CARGO_DIRS)) \
  $(patsubst %,%test.release,$(CARGO_DIRS)) \
  $(patsubst %,%fmt,$(CARGO_DIRS))

$(ALL_TARGETS):

%build.release: COMMAND := cargo build --release
%check.release: COMMAND := cargo check --release
%test.release:  COMMAND := cargo test  --release
%fmt:           COMMAND := cargo fmt

%.debug %.release %fmt:
	cached-nix-shell -I nixpkgs=channel:nixos-21.05 third-party/nix/shell.nix \
	  --run "cd $(patsubst task/%,./%,$(shell dirname $@)/) && $(COMMAND)"


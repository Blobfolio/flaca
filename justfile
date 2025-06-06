##
# Development Recipes
#
# This justfile is intended to be run from inside a Docker sandbox:
# https://github.com/Blobfolio/righteous-sandbox
#
# docker run \
#	--rm \
#	-v "{{ invocation_directory() }}":/share \
#	-it \
#	--name "righteous_sandbox" \
#	"righteous/sandbox:debian"
#
# Alternatively, you can just run cargo commands the usual way and ignore these
# recipes.
##

pkg_id1     := "flaca"
pkg_id2     := "flapfli"
pkg_name    := "Flaca"
pkg_dir1    := justfile_directory() + "/" + pkg_id1
pkg_dir2    := justfile_directory() + "/" + pkg_id2

bench_dir   := "/tmp/bench-data"
cargo_dir   := "/tmp/" + pkg_id1 + "-cargo"
cargo_bin   := cargo_dir + "/release/" + pkg_id1
doc_dir     := justfile_directory() + "/doc"
release_dir := justfile_directory() + "/release"
skel_dir    := justfile_directory() + "/skel"

export RUSTFLAGS := "-Ctarget-cpu=x86-64-v3 -Cllvm-args=--cost-kind=throughput -Clinker-plugin-lto -Clink-arg=-fuse-ld=lld"
export CC := "clang"
export CXX := "clang++"
export CFLAGS := "-Wall -Wextra -flto -march=x86-64-v3"
export CXXFLAGS := "-Wall -Wextra -flto -march=x86-64-v3"



# Build Release!
@build:
	# First let's build the Rust bit.
	cargo build \
		--bin "{{ pkg_id1 }}" \
		--release \
		--target-dir "{{ cargo_dir }}"


# Build Debian package!
@build-deb: clean credits build
	# cargo-deb doesn't support target_dir flags yet.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	mv "{{ cargo_dir }}" "{{ justfile_directory() }}/target"

	# Build the deb.
	cargo-deb \
		--no-build \
		--quiet \
		-p {{ pkg_id1 }} \
		-o "{{ release_dir }}"

	just _fix-chown "{{ release_dir }}"
	mv "{{ justfile_directory() }}/target" "{{ cargo_dir }}"


# Bench Compression.
[no-cd]
@bench-bin BIN:
	[ -f "{{ BIN }}" ] || exit 1
	just _bench-reset
	"{{ absolute_path(BIN) }}" -p --preserve-times "{{ bench_dir }}"

	# Checksum checks.
	cd "{{ bench_dir }}" && b3sum -c --quiet assets.b3


# Remove Cargo Crap.
@clean:
	# Most things go here.
	[ ! -d "{{ cargo_dir }}" ] || rm -rf "{{ cargo_dir }}"

	# But some Cargo apps place shit in subdirectories even if
	# they place *other* shit in the designated target dir. Haha.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	[ ! -d "{{ pkg_dir1 }}/target" ] || rm -rf "{{ pkg_dir1 }}/target"

	cargo update


# Clippy.
@clippy:
	clear
	cargo clippy \
		--release \
		--target-dir "{{ cargo_dir }}"


# Generate CREDITS.
@credits:
	cargo bashman -m "{{ pkg_dir1 }}/Cargo.toml" -t x86_64-unknown-linux-gnu
	just _fix-chown "{{ justfile_directory() }}/CREDITS.md"
	just _fix-chown "{{ justfile_directory() }}/release"


# Build Docs.
@doc:
	# Make the docs.
	cargo rustdoc \
		--release \
		--manifest-path "{{ pkg_dir1 }}/Cargo.toml" \
		--target-dir "{{ cargo_dir }}" \
		-- --document-private-items

	cargo rustdoc \
		--release \
		--manifest-path "{{ pkg_dir2 }}/Cargo.toml" \
		--target-dir "{{ cargo_dir }}" \
		-- --document-private-items

	# Move the docs and clean up ownership.
	[ ! -d "{{ doc_dir }}" ] || rm -rf "{{ doc_dir }}"
	mv "{{ cargo_dir }}/doc" "{{ justfile_directory() }}"
	just _fix-chown "{{ doc_dir }}"


# Test Run.
@run +ARGS:
	cargo run \
		--bin "{{ pkg_id1 }}" \
		--release \
		--target-dir "{{ cargo_dir }}" \
		-- {{ ARGS }}


# Unit tests!
@test:
	clear
	RUST_TEST_THREADS=1 cargo test \
		--target-dir "{{ cargo_dir }}"
	RUST_TEST_THREADS=1 cargo test \
		--release \
		--target-dir "{{ cargo_dir }}"


# Get/Set version.
version:
	#!/usr/bin/env bash

	# Current version.
	_ver1="$( toml get "{{ pkg_dir1 }}/Cargo.toml" package.version | \
		sed 's/"//g' )"

	# Find out if we want to bump it.
	_ver2="$( whiptail --inputbox "Set {{ pkg_name }} version:" --title "Release Version" 0 0 "$_ver1" 3>&1 1>&2 2>&3 )"

	exitstatus=$?
	if [ $exitstatus != 0 ] || [ "$_ver1" = "$_ver2" ]; then
		exit 0
	fi

	fyi success "Setting version to $_ver2."

	# Set the release version!
	just _version "{{ pkg_dir1 }}" "$_ver2"
	just _version "{{ pkg_dir2 }}" "$_ver2"


# Set version for real.
@_version DIR VER:
	[ -f "{{ DIR }}/Cargo.toml" ] || exit 1

	# Set the release version!
	toml set "{{ DIR }}/Cargo.toml" package.version "{{ VER }}" > /tmp/Cargo.toml
	just _fix-chown "/tmp/Cargo.toml"
	mv "/tmp/Cargo.toml" "{{ DIR }}/Cargo.toml"


# Reset bench.
@_bench-reset EXTRA="":
	[ ! -d "{{ bench_dir }}" ] || rm -rf "{{ bench_dir }}"
	cp -aR "{{ skel_dir }}/assets" "{{ bench_dir }}"
	[ -z "{{ EXTRA }}" ] || cp -aR "{{ skel_dir }}/pgo" "{{ bench_dir }}"


# Init dependencies.
@_init:
	# Nothing now.


# Fix file/directory permissions.
@_fix-chmod PATH:
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type f -exec chmod 0644 {} +
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type d -exec chmod 0755 {} +


# Fix file/directory ownership.
@_fix-chown PATH:
	[ ! -e "{{ PATH }}" ] || chown -R --reference="{{ justfile() }}" "{{ PATH }}"

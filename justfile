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

pkg_id      := "flaca"
pkg_name    := "Flaca"
pkg_dir1    := justfile_directory() + "/src"

bench_dir   := "/tmp/bench-data"
cargo_dir   := "/tmp/" + pkg_id + "-cargo"
cargo_bin   := cargo_dir + "/x86_64-unknown-linux-gnu/release/" + pkg_id
release_dir := justfile_directory() + "/release"
skel_dir    := justfile_directory() + "/skel"

rustflags   := "-C link-arg=-s"



# Build Release!
@build: clean
	# First let's build the Rust bit.
	RUSTFLAGS="--emit asm {{ rustflags }}" cargo build \
		--bin "{{ pkg_id }}" \
		--release \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


# Build Debian package!
@build-deb: credits build _init-zopflipng
	# Do completions/man.
	cargo bashman -m "{{ justfile_directory() }}/Cargo.toml"

	# cargo-deb doesn't support target_dir flags yet.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	mv "{{ cargo_dir }}" "{{ justfile_directory() }}/target"

	# Build the deb.
	cargo-deb \
		--no-build \
		-p {{ pkg_id }} \
		-o "{{ justfile_directory() }}/release"

	just _fix-chown "{{ release_dir }}"
	mv "{{ justfile_directory() }}/target" "{{ cargo_dir }}"


# Check Release!
@check:
	# First let's build the Rust bit.
	RUSTFLAGS="{{ rustflags }}" cargo check \
		--release \
		--all-features \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


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
	RUSTFLAGS="{{ rustflags }}" cargo clippy \
		--release \
		--all-features \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


# Generate CREDITS.
@credits:
	# Update CREDITS.html.
	cargo about \
		-m "{{ justfile_directory() }}/Cargo.toml" \
		generate \
		"{{ release_dir }}/credits/about.hbs" > "{{ justfile_directory() }}/CREDITS.md"

	just _fix-chown "{{ justfile_directory() }}/CREDITS.md"


# Test Run.
@run +ARGS:
	RUSTFLAGS="{{ rustflags }}" cargo run \
		--bin "{{ pkg_id }}" \
		--release \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}" \
		-- {{ ARGS }}


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


# Set version for real.
@_version DIR VER:
	[ -f "{{ DIR }}/Cargo.toml" ] || exit 1

	# Set the release version!
	toml set "{{ DIR }}/Cargo.toml" package.version "{{ VER }}" > /tmp/Cargo.toml
	just _fix-chown "/tmp/Cargo.toml"
	mv "/tmp/Cargo.toml" "{{ DIR }}/Cargo.toml"


# Reset bench.
@_bench-reset:
	[ ! -d "{{ bench_dir }}" ] || rm -rf "{{ bench_dir }}"
	cp -aR "{{ skel_dir }}/assets" "{{ bench_dir }}"


# Init dependencies.
@_init:
	# We need beta until 1.51 is stable.
	rustup default beta
	rustup component add clippy

	[ -f "{{ release_dir }}/zopflipng" ] || just _init-zopflipng
	[ ! -f "{{ justfile_directory() }}/Cargo.lock" ] || rm "{{ justfile_directory() }}/Cargo.lock"
	cargo update
	cargo outdated


# Init (build) Zopflipng.
@_init-zopflipng:
	# Start fresh!
	[ ! -d "/tmp/zopfli" ] || rm -rf "/tmp/zopfli"

	# Clone and build it.
	git clone https://github.com/google/zopfli "/tmp/zopfli"
	cd "/tmp/zopfli" && make zopflipng
	[ -f "/tmp/zopfli/zopflipng" ] || fyi error --exit 1 "Failed to make Zopflipng."

	# Move the file, set permissions, etc.
	mv "/tmp/zopfli/zopflipng" "{{ release_dir }}/"
	just _fix-chown "{{ release_dir }}/zopflipng"
	chmod 755 "{{ release_dir }}/zopflipng"

	# Make our dev lives easier by making sure the /var/lib/flaca copy is set.
	[ -d "/var/lib/flaca" ] || mkdir -p "/var/lib/flaca"
	cp -a "{{ release_dir }}/zopflipng" "/var/lib/flaca/"

	# Clean up.
	rm -rf "/tmp/zopfli"


# Fix file/directory permissions.
@_fix-chmod PATH:
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type f -exec chmod 0644 {} +
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type d -exec chmod 0755 {} +


# Fix file/directory ownership.
@_fix-chown PATH:
	[ ! -e "{{ PATH }}" ] || chown -R --reference="{{ justfile() }}" "{{ PATH }}"

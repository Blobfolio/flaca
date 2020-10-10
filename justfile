##
# Development Recipes
#
# This requires Just: https://github.com/casey/just
#
# To see possible tasks, run:
# just --list
##

pkg_id      := "flaca"
pkg_name    := "Flaca"
pkg_dir1    := justfile_directory() + "/flaca"
pkg_dir2    := justfile_directory() + "/flaca_core"

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
@build-deb: build-man build
	# cargo-deb doesn't support target_dir flags yet.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	mv "{{ cargo_dir }}" "{{ justfile_directory() }}/target"

	# First let's build the Rust bit.
	cargo-deb \
		--no-build \
		-p {{ pkg_id }} \
		-o "{{ justfile_directory() }}/release"

	just _fix-chown "{{ release_dir }}"
	mv "{{ justfile_directory() }}/target" "{{ cargo_dir }}"


# Build Man.
@build-man:
	# Pre-clean.
	find "{{ skel_dir }}/man" -name "{{ pkg_id }}.1*" -type f -delete

	# Build a quickie version with the unsexy help so help2man can parse it.
	RUSTFLAGS="{{ rustflags }}" cargo build \
		--bin "{{ pkg_id }}" \
		--release \
		--all-features \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"

	# Use help2man to make a crappy MAN page.
	help2man -o "{{ skel_dir }}/man/{{ pkg_id }}.1" \
		-N "{{ cargo_bin }}"

	sed -i -Ee \
		's#^(MozJPEG|Oxipng|Zopflipng) +(<[^>]+>)#.TP\n\1\n\2#g' \
		"{{ skel_dir }}/man/{{ pkg_id }}.1"

	sed -i -e ':a' -e 'N' -e '$!ba' -Ee \
		"s#.SS \"OPTIMIZERS USED:\"[\n].IP#.SS \"OPTIMIZERS USED:\"#g" \
		"{{ skel_dir }}/man/{{ pkg_id }}.1"

	# Gzip it and reset ownership.
	gzip -k -f -9 "{{ skel_dir }}/man/{{ pkg_id }}.1"
	just _fix-chown "{{ skel_dir }}/man"


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
	[ ! -d "{{ pkg_dir2 }}/target" ] || rm -rf "{{ pkg_dir2 }}/target"


# Clippy.
@clippy:
	clear
	RUSTFLAGS="{{ rustflags }}" cargo clippy \
		--release \
		--all-features \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


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
	_ver1="$( toml get "{{ pkg_dir2 }}/Cargo.toml" package.version | \
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
@_bench-reset:
	[ ! -d "{{ bench_dir }}" ] || rm -rf "{{ bench_dir }}"
	cp -aR "{{ skel_dir }}/assets" "{{ bench_dir }}"


# Init dependencies.
@_init:
	[ -d "{{ justfile_directory() }}/mozjpeg_sys" ] || just _init-mozjpeg
	[ ! -f "{{ justfile_directory() }}/Cargo.lock" ] || rm "{{ justfile_directory() }}/Cargo.lock"
	cargo update


# Install Bindgen and Dependencies
@_init-bindgen:
	apt-get update
	apt-fast install \
		clang \
		libclang-dev \
		libjpeg-dev \
		libpng-dev \
		llvm-dev
	cargo install bindgen

	# cmake -G"Unix Makefiles"

	# bindgen --disable-name-namespacing --no-derive-copy --no-derive-debug --no-layout-tests --no-prepend-enum-name  --use-core
	# -o raw-bindgen.rs jpegtran-bindgen.h


# Init Mozjpeg-Sys.
@_init-mozjpeg:
	# Start fresh!
	[ ! -d "{{ justfile_directory() }}/mozjpeg_sys" ] || rm -rf "{{ justfile_directory() }}/mozjpeg_sys"

	# Clone the main repo. New commits could potentially break our patch, so
	# let's checkout to a specific place that is known to work.
	git clone -n \
		https://github.com/kornelski/mozjpeg-sys.git \
		"{{ justfile_directory() }}/mozjpeg_sys" \
		&& cd "{{ justfile_directory() }}/mozjpeg_sys" \
		&& git checkout 3d7e9ed4fd66d789fcb99821c3f260361da6a2a3 \
		&& git submodule update --init

	# Patch it.
	cd "{{ justfile_directory() }}/mozjpeg_sys" \
		&& git apply ../skel/mozjpeg_sys/jpegtran.patch

	# Copy our extra lib exports.
	cp -a \
		"{{ skel_dir }}/mozjpeg_sys/jpegtran.rs" \
		"{{ justfile_directory() }}/mozjpeg_sys/src"

	just _fix-chown "{{ justfile_directory() }}/mozjpeg_sys"


# Fix file/directory permissions.
@_fix-chmod PATH:
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type f -exec chmod 0644 {} +
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type d -exec chmod 0755 {} +


# Fix file/directory ownership.
@_fix-chown PATH:
	[ ! -e "{{ PATH }}" ] || chown -R --reference="{{ justfile() }}" "{{ PATH }}"

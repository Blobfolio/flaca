##
# Development Recipes
#
# This requires Just: https://github.com/casey/just
#
# To see possible tasks, run:
# just --list
##

cargo_dir     := "/tmp/flaca-cargo"
data_dir      := "/tmp/bench-data"
debian_dir    := "/tmp/flaca-release/flaca"
release_dir   := justfile_directory() + "/release"

build_ver     := "1"


# Benchmark Directory Comparisons.
@bench: build
	[ ! -d "{{ data_dir }}" ] || rm -rf "{{ data_dir }}"
	cp -aR "{{ justfile_directory() }}/test-assets" "{{ data_dir }}"
	"{{ cargo_dir }}/release/flaca" -p "{{ data_dir }}"


# Build Release!
@build:
	# First let's build the Rust bit.
	RUSTFLAGS="-C link-arg=-s" cargo build \
		--release \
		--target-dir "{{ cargo_dir }}"


# Build Debian Package.
@build-debian: build
	[ ! -e "{{ debian_dir }}" ] || rm -rf "{{ debian_dir }}"
	mkdir -p "{{ debian_dir }}/DEBIAN"
	mkdir -p "{{ debian_dir }}/etc/bash_completion.d"
	mkdir -p "{{ debian_dir }}/usr/bin"
	mkdir -p "{{ debian_dir }}/usr/share/man/man1"

	# Steal the version from Cargo.toml really quick.
	cat "{{ justfile_directory() }}/flaca/Cargo.toml" | grep version | head -n 1 | sed 's/[^0-9\.]//g' > "/tmp/VERSION"

	# Copy the application.
	cp -a "{{ cargo_dir }}/release/flaca" "{{ debian_dir }}/usr/bin"
	chmod 755 "{{ debian_dir }}/usr/bin/flaca"
	strip "{{ debian_dir }}/usr/bin/flaca"

	# Generate completions.
	cp -a "{{ cargo_dir }}/flaca.bash" "{{ debian_dir }}/etc/bash_completion.d"
	chmod 644 "{{ debian_dir }}/etc/bash_completion.d/flaca.bash"

	# Set up the control file.
	cp -a "{{ release_dir }}/skel/control" "{{ debian_dir }}/DEBIAN"
	sed -i "s/VERSION/$( cat "/tmp/VERSION" )-{{ build_ver }}/g" "{{ debian_dir }}/DEBIAN/control"
	sed -i "s/SIZE/$( du -scb "{{ debian_dir }}/usr" | tail -n 1 | awk '{print $1}' )/g" "{{ debian_dir }}/DEBIAN/control"

	# Generate the manual.
	just _build-man

	# Build the Debian package.
	chown -R root:root "{{ debian_dir }}"
	cd "$( dirname "{{ debian_dir }}" )" && dpkg-deb --build flaca
	chown --reference="{{ justfile() }}" "$( dirname "{{ debian_dir }}" )/flaca.deb"

	# And a touch of clean-up.
	mv "$( dirname "{{ debian_dir }}" )/flaca.deb" "{{ release_dir }}/flaca_$( cat "/tmp/VERSION" )-{{ build_ver }}.deb"
	rm -rf "/tmp/VERSION" "{{ debian_dir }}"


# Build MAN page.
@_build-man:
	# Most of it can come straight from the help screen.
	help2man -N \
		"{{ debian_dir }}/usr/bin/flaca" > "{{ debian_dir }}/usr/share/man/man1/flaca.1"

	# Fix a few formatting quirks.
	sed -i -e ':a' -e 'N' -e '$!ba' -Ee \
		"s#Flaca [0-9\.]+[\n]Blobfolio, LLC. <hello@blobfolio.com>[\n]##g" \
		"{{ debian_dir }}/usr/share/man/man1/flaca.1"
	sed -i -Ee 's#^(Jpegoptim|MozJPEG|Oxipng|Zopflipng|Pngout) +(<[^>]+>)#.TP\n\1\n\2#g' \
		"{{ debian_dir }}/usr/share/man/man1/flaca.1"
	sed -i -e ':a' -e 'N' -e '$!ba' -Ee \
		"s#.SS \"OPTIMIZERS USED:\"[\n].IP#.SS \"OPTIMIZERS USED:\"#g" \
		"{{ debian_dir }}/usr/share/man/man1/flaca.1"

	# Wrap up by gzipping to save some space.
	gzip -9 "{{ debian_dir }}/usr/share/man/man1/flaca.1"


# Get/Set Flaca version.
version:
	#!/usr/bin/env bash

	# Current version.
	_ver1="$( cat "{{ justfile_directory() }}/flaca/Cargo.toml" | \
		grep version | \
		head -n 1 | \
		sed 's/[^0-9\.]//g' )"

	# Find out if we want to bump it.
	_ver2="$( whiptail --inputbox "Set Flaca version:" --title "Release Version" 0 0 "$_ver1" 3>&1 1>&2 2>&3 )"

	exitstatus=$?
	if [ $exitstatus != 0 ] || [ "$_ver1" = "$_ver2" ]; then
		exit 0
	fi

	fyi success "Setting plugin version to $_ver2."

	# Set the release version!
	just _version "{{ justfile_directory() }}/flaca/Cargo.toml" "$_ver2" >/dev/null 2>&1
	just _version "{{ justfile_directory() }}/flaca_core/Cargo.toml" "$_ver2" >/dev/null 2>&1


# Truly set version.
_version TOML VER:
	#!/usr/bin/env php
	<?php
	if (! is_file("{{ TOML }}") || ! preg_match('/^\d+.\d+.\d+$/', "{{ VER }}")) {
		exit(1);
	}

	$content = file_get_contents("{{ TOML }}");
	$content = explode("\n", $content);
	$section = null;

	foreach ($content as $k=>$v) {
		if (\preg_match('/^\[[^\]]+\]$/', $v)) {
			$section = $v;
			continue;
		}
		elseif ('[package]' === $section && 0 === \strpos($v, 'version')) {
			$content[$k] = \sprintf(
				'version = "%s"',
				"{{ VER }}"
			);
			break;
		}
	}

	$content = implode("\n", $content);
	file_put_contents("{{ TOML }}", $content);


# Init dependencies.
@_init:
	echo ""

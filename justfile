##
# Development Recipes
#
# This requires Just: https://github.com/casey/just
#
# To see possible tasks, run:
# just --list
##

cargo_dir     := "/tmp/flaca-cargo"
debian_dir    := "/tmp/flaca-release/flaca"
release_dir   := justfile_directory() + "/release"

build_ver     := "2"



# Build Release!
@build: test
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
	"{{ debian_dir }}/usr/bin/flaca" --completions . > "{{ debian_dir }}/etc/bash_completion.d/flaca.bash"
	chmod 644 "{{ debian_dir }}/etc/bash_completion.d/flaca.bash"

	# Set up the control file.
	cp -a "{{ release_dir }}/skel/conffiles" "{{ debian_dir }}/DEBIAN"
	cp -a "{{ release_dir }}/skel/flaca.yml" "{{ debian_dir }}/etc"
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
	help2man -N \
		"{{ debian_dir }}/usr/bin/flaca" > "{{ debian_dir }}/usr/share/man/man1/flaca.1"
	sed -i -e ':a' -e 'N' -e '$!ba' -Ee \
		"s#Flaca [0-9\.]+[\n]Blobfolio, LLC. <hello@blobfolio.com>[\n]##g" \
		"{{ debian_dir }}/usr/share/man/man1/flaca.1"
	sed -i -Ee 's#^(Jpegoptim|MozJPEG|Oxipng|Zopflipng|Pngout) +(<[^>]+>)#.TP\n\1\n\2#g' \
		"{{ debian_dir }}/usr/share/man/man1/flaca.1"
	sed -i -e ':a' -e 'N' -e '$!ba' -Ee \
		"s#.SS \"GLOBAL CONFIGURATION:\"[\n].IP#.SS \"GLOBAL CONFIGURATION:\"\n.TP#g" \
		"{{ debian_dir }}/usr/share/man/man1/flaca.1"
	sed -i -e ':a' -e 'N' -e '$!ba' -Ee \
		"s#.SS \"SUPPORTED OPTIMIZERS:\"[\n].IP#.SS \"SUPPORTED OPTIMIZERS:\"#g" \
		"{{ debian_dir }}/usr/share/man/man1/flaca.1"



# Run Normal Unit Tests.
@test:
	cargo test --target-dir "{{ cargo_dir }}"


# Run All Unit Tests (Even Slow Ones).
@test-all:
	cargo test --target-dir "{{ cargo_dir }}"
	cargo test --target-dir "{{ cargo_dir }}" -- --ignored


# Init dependencies.
@_init:
	apt-get update -qq
	apt-fast install -qq -y \
		help2man \
		jpegoptim \
		libjpeg-turbo-progs \
		mozjpeg \
		oxipng \
		pngout \
		zopflipng

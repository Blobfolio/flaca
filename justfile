##
# Development Recipes
#
# This requires Just: https://github.com/casey/just
#
# To see possible tasks, run:
# just --list
##

cargo_dir     := "/tmp/flaca-cargo"
release_dir   := justfile_directory() + "/release"
debian_dir    := release_dir + "/flaca"

build_ver     := "1"



# Build Release!
@build: test
	# First let's build the Rust bit.
	RUSTFLAGS="-C link-arg=-s" cargo build \
		--release \
		--target-dir "{{ cargo_dir }}"


# Build Debian Package.
@build-debian: build
	# Switch ownership temporarily to make operations easier.
	chown -R --reference="{{ justfile() }}" "{{ debian_dir }}"

	# Steal the version from Cargo.toml really quick.
	cat "{{ justfile_directory() }}/flaca/Cargo.toml" | grep version | head -n 1 | sed 's/[^0-9\.]//g' > "{{ release_dir }}/VERSION"

	# Copy the application.
	cp -a "{{ cargo_dir }}/release/flaca" "{{ debian_dir }}/usr/bin"
	chmod 755 "{{ debian_dir }}/usr/bin/flaca"
	strip "{{ debian_dir }}/usr/bin/flaca"

	# Set up the control file.
	cp -a "{{ release_dir }}/skel/control" "{{ debian_dir }}/DEBIAN"
	sed -i "s/VERSION/$( cat "{{ release_dir }}/VERSION" )-{{ build_ver }}/g" "{{ debian_dir }}/DEBIAN/control"
	sed -i "s/SIZE/$( du -scb "{{ debian_dir }}/usr" | tail -n 1 | awk '{print $1}' )/g" "{{ debian_dir }}/DEBIAN/control"

	# Build the Debian package.
	chown -R root:root "{{ debian_dir }}"
	cd "{{ release_dir }}" && dpkg-deb --build flaca
	chown --reference="{{ justfile() }}" "{{ release_dir }}/flaca.deb"

	# And a touch of clean-up.
	mv "{{ release_dir }}/flaca.deb" "{{ release_dir }}/flaca_$( cat "{{ release_dir }}/VERSION" )-{{ build_ver }}.deb"
	rm "{{ release_dir }}/VERSION"


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
		jpegoptim \
		mozjpeg \
		oxipng \
		pngout \
		zopflipng

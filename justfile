##
# Development Recipes
#
# This requires Just: https://github.com/casey/just
#
# To see possible tasks, run:
# just --list
##



root_dir      := `echo $PWD`
release_dir   := root_dir + "/release"
debian_dir    := release_dir + "/flaca"

build_ver     := "1"



# Build Release!
@build: test
	# First let's build the Rust bit.
	RUSTFLAGS="-C link-arg=-s" cargo build --release


# Build Debian Package.
@build-debian: build
	# Switch ownership temporarily to make operations easier.
	sudo chown -R --reference="{{ root_dir }}/justfile" "{{ debian_dir }}"

	# Steal the version from Cargo.toml really quick.
	cat "{{ root_dir }}/flaca/Cargo.toml" | grep version | head -n 1 | sed 's/[^0-9\.]//g' > "{{ release_dir }}/VERSION"

	# Copy the application.
	cp -a "{{ root_dir }}/target/release/flaca" "{{ debian_dir }}/usr/bin"
	chmod 755 "{{ debian_dir }}/usr/bin/flaca"
	strip "{{ debian_dir }}/usr/bin/flaca"

	# Set up the control file.
	cp -a "{{ release_dir }}/skel/control" "{{ debian_dir }}/DEBIAN"
	sed -i "s/VERSION/$( cat "{{ release_dir }}/VERSION" )-{{ build_ver }}/g" "{{ debian_dir }}/DEBIAN/control"
	sed -i "s/SIZE/$( du -scb "{{ debian_dir }}/usr" | tail -n 1 | awk '{print $1}' )/g" "{{ debian_dir }}/DEBIAN/control"

	# Build the Debian package.
	sudo chown -R root:root "{{ debian_dir }}"
	cd "{{ release_dir }}" && dpkg-deb --build flaca
	sudo chown --reference="{{ root_dir }}/justfile" "{{ release_dir }}/flaca.deb"

	# And a touch of clean-up.
	mv "{{ release_dir }}/flaca.deb" "{{ release_dir }}/flaca_$( cat "{{ release_dir }}/VERSION" )-{{ build_ver }}.deb"
	rm "{{ release_dir }}/VERSION"


# Run Normal Unit Tests.
@test:
	cargo test


# Run All Unit Tests (Even Slow Ones).
@test-all:
	cargo test
	cargo test -- --ignored

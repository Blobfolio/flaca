##
# Building Flaca
#
# This requires "just". See https://github.com/casey/just for more
# details.
#
#
# USAGE:
#
# just --list
# just <task>
##

base_dir      = invocation_directory()
dist_dir      = base_dir + "/dist"
target        = base_dir + "/target/release/flaca"

jpegoptim_url = "https://github.com/tjko/jpegoptim.git"
mozjpeg_url   = "https://github.com/mozilla/mozjpeg.git"
oxipng_url    = "https://github.com/shssoichiro/oxipng.git"
pngout_url    = "http://static.jonof.id.au/dl/kenutils/pngout-20150319-linux-static.tar.gz"
zopflipng_url = "https://github.com/google/zopfli.git"

build:
	@echo "\e[34m-------------------------------------\e[0m"
	@echo "\e[34m Build Flaca from the Cargo sources.\e[0m"
	@echo "\e[34m-------------------------------------\e[0m"

	@echo ""
	@echo "\e[95;1m• Dependency Checks\e[0m"
	# cargo
	@if [ ! $( command -v cargo ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'cargo'." && exit 1; fi

	@echo ""
	@echo "\e[95;1m• Building\e[0m"
	@RUSTFLAGS="-C link-arg=-s" cargo build --release

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@echo ""

build-debian: build
	@echo "\e[34m----------------------------------------\e[0m"
	@echo "\e[34m Compile installable .deb package file.\e[0m"
	@echo "\e[34m----------------------------------------\e[0m"

	@echo ""
	@echo "\e[95;1m• Dependency Checks\e[0m"
	# dpkg-deb
	@if [ ! $( command -v dpkg-deb ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'dpkg-deb'." && exit 1; fi

	@echo ""
	@echo "\e[95;1m• Set Up Package\e[0m"
	# Patch version and size in "control" package file.
	sudo cp "{{ dist_dir }}/skel/control" "{{ dist_dir }}/deb/DEBIAN/control"
	sudo sed -i "s#VERSION#`{{ target }} -V | cut -d' ' -f 2`#g" "{{ dist_dir }}/deb/DEBIAN/control"
	sudo sed -i "s#SIZE#`wc -c {{ target }} | cut -d' ' -f 1`#g" "{{ dist_dir }}/deb/DEBIAN/control"
	# Copy binary to package source.
	sudo cp "{{ target }}" "{{ dist_dir }}/deb/usr/bin/flaca"
	sudo chown -R root:root "{{ dist_dir }}/deb"
	sudo chmod 755 "{{ dist_dir }}/deb/usr/bin/flaca"

	@echo ""
	@echo "\e[95;1m• Build Package\e[0m"
	cd "{{ dist_dir }}"; dpkg-deb --build deb
	mv "{{ dist_dir }}/deb.deb" "{{ dist_dir }}/flaca_`{{ target }} -V | cut -d' ' -f 2`.deb"

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@echo ""

install: install-flaca install-jpegoptim install-mozjpeg install-oxipng install-zopflipng install-pngout
	@echo "\e[34m----------------------------\e[0m"
	@echo "\e[34m Install Goddamn Everything\e[0m"
	@echo "\e[34m----------------------------\e[0m"

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@echo ""

install-flaca: build
	@echo "\e[34m------------------\e[0m"
	@echo "\e[34m Install Flaca!!!\e[0m"
	@echo "\e[34m------------------\e[0m"

	@echo ""
	@echo "\e[95;1m• Copy Executable\e[0m"
	sudo cp {{ target }} /usr/bin/flaca
	sudo chmod 755 /usr/bin/flaca

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@/usr/bin/flaca -V
	@echo ""

install-jpegoptim:
	@echo "\e[34m--------------------------------\e[0m"
	@echo "\e[34m Compile jpegoptim from source.\e[0m"
	@echo "\e[34m--------------------------------\e[0m"

	@echo ""
	@echo "\e[95;1m• Dependency Checks\e[0m"
	# git
	@if [ ! $( command -v git ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'git'." && exit 1; fi
	# make
	@if [ ! $( command -v make ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'make'." && exit 1; fi

	@echo ""
	@echo "\e[95;1m• Pre-Clean\e[0m"
	[ ! -e /tmp/jpegoptim ] || sudo rm -rf /tmp/jpegoptim
	[ ! -e /usr/share/flaca/jpegoptim ] || sudo rm /usr/share/flaca/jpegoptim
	[ ! -e /usr/share/flaca/src/jpegoptim ] || sudo rm -rf /usr/share/flaca/src/jpegoptim

	@echo ""
	@echo "\e[95;1m• Fetch Sources\e[0m"
	git clone {{ jpegoptim_url }} /tmp/jpegoptim

	@echo ""
	@echo "\e[95;1m• Build & Install\e[0m"
	cd /tmp/jpegoptim; ./configure --prefix=/usr/share/flaca/src/jpegoptim
	cd /tmp/jpegoptim; make; make strip; sudo make install
	cd /usr/share/flaca; sudo ln -s src/jpegoptim/bin/jpegoptim jpegoptim

	@echo ""
	@echo "\e[95;1m• Post-Clean\e[0m"
	sudo rm -rf /tmp/jpegoptim

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@/usr/share/flaca/jpegoptim -V
	@echo ""

install-mozjpeg:
	@echo "\e[34m------------------------------\e[0m"
	@echo "\e[34m Compile MozJPEG from source.\e[0m"
	@echo "\e[34m------------------------------\e[0m"

	@echo ""
	@echo "\e[95;1m• Dependency Checks\e[0m"
	# cmake
	@if [ ! $( command -v cmake ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'cmake'." && exit 1; fi
	# git
	@if [ ! $( command -v git ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'git'." && exit 1; fi
	# make
	@if [ ! $( command -v make ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'make'." && exit 1; fi

	@echo ""
	@echo "\e[95;1m• Pre-Clean\e[0m"
	[ ! -e /tmp/mozjpeg ] || sudo rm -rf /tmp/mozjpeg
	[ ! -e /usr/share/flaca/jpegtran ] || sudo rm /usr/share/flaca/jpegtran
	[ ! -e /usr/share/flaca/src/mozjpeg ] || sudo rm -rf /usr/share/flaca/src/mozjpeg

	@echo ""
	@echo "\e[95;1m• Fetch Sources\e[0m"
	git clone {{ mozjpeg_url }} /tmp/mozjpeg

	@echo ""
	@echo "\e[95;1m• Build & Install\e[0m"
	mkdir /tmp/mozjpeg/build
	cd /tmp/mozjpeg/build; cmake -G"Unix Makefiles" -DCMAKE_INSTALL_PREFIX=/usr/share/flaca/src/mozjpeg ../
	cd /tmp/mozjpeg/build; make; sudo make install
	cd /usr/share/flaca; sudo ln -s src/mozjpeg/bin/jpegtran jpegtran

	@echo ""
	@echo "\e[95;1m• Post-Clean\e[0m"
	sudo rm -rf /tmp/mozjpeg

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@/usr/share/flaca/jpegtran -version
	@echo ""

install-oxipng:
	@echo "\e[34m-----------------------------\e[0m"
	@echo "\e[34m Compile Oxipng from source.\e[0m"
	@echo "\e[34m-----------------------------\e[0m"

	@echo ""
	@echo "\e[95;1m• Dependency Checks\e[0m"
	# cargo
	@if [ ! $( command -v cargo ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'cargo'." && exit 1; fi
	# git
	@if [ ! $( command -v git ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'git'." && exit 1; fi

	@echo ""
	@echo "\e[95;1m• Pre-Clean\e[0m"
	[ ! -e /tmp/oxipng ] || sudo rm -rf /tmp/oxipng
	[ ! -e /usr/share/flaca/oxipng ] || sudo rm /usr/share/flaca/oxipng

	@echo ""
	@echo "\e[95;1m• Fetch Sources\e[0m"
	git clone {{ oxipng_url }} /tmp/oxipng

	@echo ""
	@echo "\e[95;1m• Build & Install\e[0m"
	# Add optimized release profile instructions to Cargo.toml.
	echo "\n[profile.release]\nlto = true\npanic = \"abort\"\nopt-level = 3\n" >> /tmp/oxipng/Cargo.toml
	# Build as usual.
	cd /tmp/oxipng; RUSTFLAGS="-C link-arg=-s -C target-cpu=native" cargo build --release
	[ -e /usr/share/flaca ] || sudo mkdir /usr/share/flaca
	sudo cp /tmp/oxipng/target/release/oxipng /usr/share/flaca/oxipng
	sudo chmod 755 /usr/share/flaca/oxipng

	@echo ""
	@echo "\e[95;1m• Post-Clean\e[0m"
	sudo rm -rf /tmp/oxipng

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@/usr/share/flaca/oxipng -V
	@echo ""

install-pngout:
	@echo "\e[34m-----------------------------------------\e[0m"
	@echo "\e[34m Download pngout (no sources available).\e[0m"
	@echo "\e[34m-----------------------------------------\e[0m"

	@echo ""
	@echo "\e[95;1m• Dependency Checks\e[0m"
	# Linux
	@if [ {{ os() }} != "linux" ]; then echo "\e[31;1mError:\e[0m You must install pngout via 'brew', etc." && exit 1; fi
	# wget
	@if [ ! $( command -v cargo ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'cargo'." && exit 1; fi

	@echo ""
	@echo "\e[95;1m• Download & Install\e[0m"
	wget -O /tmp/pngout {{ pngout_url }}
	chmod 755 /tmp/pngout
	[ -e /usr/share/flaca ] || sudo mkdir /usr/share/flaca
	sudo mv /tmp/pngout /usr/share/flaca/pngout

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@echo ""

install-zopflipng:
	@echo "\e[34m--------------------------------\e[0m"
	@echo "\e[34m Compile Zopflipng from source.\e[0m"
	@echo "\e[34m--------------------------------\e[0m"

	@echo ""
	@echo "\e[95;1m• Dependency Checks\e[0m"
	# git
	@if [ ! $( command -v git ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'git'." && exit 1; fi
	# make
	@if [ ! $( command -v make ) ]; then echo "\e[31;1mError:\e[0m Missing build dep 'make'." && exit 1; fi

	@echo ""
	@echo "\e[95;1m• Pre-Clean\e[0m"
	[ ! -e /tmp/zopflipng ] || sudo rm -rf /tmp/zopflipng
	[ ! -e /usr/share/flaca/zopflipng ] || sudo rm /usr/share/flaca/zopflipng

	@echo ""
	@echo "\e[95;1m• Fetch Sources\e[0m"
	git clone {{ zopflipng_url }} /tmp/zopflipng

	@echo ""
	@echo "\e[95;1m• Build & Install\e[0m"
	cd /tmp/zopflipng; make zopflipng
	[ -e /usr/share/flaca ] || sudo mkdir /usr/share/flaca
	sudo cp /tmp/zopflipng/zopflipng /usr/share/flaca/zopflipng
	sudo chmod 755 /usr/share/flaca/zopflipng

	@echo ""
	@echo "\e[95;1m• Post-Clean\e[0m"
	sudo rm -rf /tmp/zopflipng

	@echo ""
	@echo "\e[92;1m• Done!\e[0m"
	@echo ""

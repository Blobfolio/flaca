##
# Flaca Makefile
#
# This contains several scripts for development and release packaging.
# To run them, install "just" <https://github.com/casey/just> and type:
#
# just --list
# just <task>
#
# ©2019 Blobfolio, LLC <hello@blobfolio.com>
# WTFPL <http://www.wtfpl.net>
##

base_dir      = invocation_directory()
dist_dir      = base_dir + "/dist"
target        = base_dir + "/target/release/flaca"
flaca_dir     = "/usr/share/flaca"
working_dir   = "/tmp/flaca"

jpegoptim_url = "https://github.com/tjko/jpegoptim.git"
mozjpeg_url   = "https://github.com/mozilla/mozjpeg.git"
oxipng_url    = "https://github.com/shssoichiro/oxipng.git"
pngout_url    = "http://static.jonof.id.au/dl/kenutils/pngout-20150319-linux-static.tar.gz"
zopflipng_url = "https://github.com/google/zopfli.git"



# Build Flaca.
@build:
	just _header "build"

	just _info "Checking dependencies."
	just _require "cargo"

	just _info "Building 'flaca'."
	RUSTFLAGS="-C link-arg=-s" cargo build --release

	just _success "build"



# Compile an installable .deb package for Flaca.
@build-debian: build
	just _header "build-debian"

	just _info "Checking dependencies."
	just _require "dpkg-deb"

	just _info "Patch control version and size."
	@sudo cp "{{ dist_dir }}/skel/control" "{{ dist_dir }}/deb/DEBIAN/control"
	just _replace "{{ dist_dir }}/deb/DEBIAN/control" "VERSION" "`{{ target }} -V | cut -d' ' -f 2`"
	just _replace "{{ dist_dir }}/deb/DEBIAN/control" "SIZE" "`wc -c {{ target }} | cut -d' ' -f 1`"

	just _info "Copy binary to package sources."
	@sudo cp "{{ target }}" "{{ dist_dir }}/deb/usr/bin/flaca"
	@sudo chown -R root:root "{{ dist_dir }}/deb"
	@sudo chmod 755 "{{ dist_dir }}/deb/usr/bin/flaca"

	just _info "Build Debian package."
	@cd "{{ dist_dir }}"; dpkg-deb --build deb
	@mv "{{ dist_dir}}/deb.deb" "{{ dist_dir}}/flaca_`{{ target }} -V | cut -d' ' -f 2`.deb"

	just _success "build-debian"



# Build and install Flaca and all dependencies.
@install: install-flaca install-jpegoptim install-mozjpeg install-oxipng install-zopflipng install-pngout
	just _success "install"



# Build and install Flaca.
@install-flaca: build
	just _header "install-flaca"

	just _info "Copy executable to PATH."
	@sudo cp {{ target }} /usr/bin/flaca
	@sudo chmod 755 /usr/bin/flaca

	just _success "install-flaca"



# Build and install jpegoptim.
@install-jpegoptim:
	just _header "install-jpegoptim"

	just _info "Checking dependencies."
	just _require "git"
	just _require "make"

	just _info "Pre-install cleaning."
	just _remove "{{ flaca_dir }}/jpegoptim"
	just _remove "{{ flaca_dir }}/src/jpegoptim"
	just _make_flaca_dir

	just _info "Fetching source."
	just _fetch {{ jpegoptim_url }}

	just _info "Building jpegoptim."
	@{ \
		cd "{{ working_dir }}"; \
		./configure --prefix={{ flaca_dir }}/src/jpegoptim; \
		make; \
		make strip; \
		sudo make install; \
	}
	@cd {{ flaca_dir }}; sudo ln -s src/jpegoptim/bin/jpegoptim jpegoptim

	just _info "Post-install cleaning."
	just _remove "{{ working_dir }}"

	just _success "install-jpegoptim"



# Build and install MozJPEG.
@install-mozjpeg:
	just _header "install-mozjpeg"

	just _info "Checking dependencies."
	just _require "cmake"
	just _require "git"
	just _require "make"

	just _info "Pre-install cleaning."
	just _remove "{{ flaca_dir }}/jpegtran"
	just _remove "{{ flaca_dir }}/src/mozjpeg"
	just _make_flaca_dir

	just _info "Fetching source."
	just _fetch {{ mozjpeg_url }}

	just _info "Building MozJPEG."
	@{ \
		mkdir "{{ working_dir }}/build"; \
		cd "{{ working_dir }}/build"; \
		cmake -G"Unix Makefiles" -DCMAKE_INSTALL_PREFIX={{ flaca_dir }}/src/mozjpeg ../; \
		make; \
		sudo make install; \
	}
	@cd {{ flaca_dir }}; sudo ln -s src/mozjpeg/bin/jpegtran jpegtran

	just _info "Post-install cleaning."
	just _remove "{{ working_dir }}"

	just _success "install-mozjpeg"



# Build and install oxipng.
@install-oxipng:
	just _header "install-oxipng"

	just _info "Checking dependencies."
	just _require "cargo"
	just _require "git"

	just _info "Pre-install cleaning."
	just _remove "{{ flaca_dir }}/oxipng"
	just _make_flaca_dir

	just _info "Fetching source."
	just _fetch {{ oxipng_url }}

	just _info "Optimizing release profile."
	@echo "\n[profile.release]\nlto = true\npanic = \"abort\"\nopt-level = 3\n" >> "{{ working_dir }}/Cargo.toml"

	just _info "Building Oxipng."
	@cd "{{ working_dir }}"; RUSTFLAGS="-C link-arg=-s -C target-cpu=native" cargo build --release
	@sudo cp "{{ working_dir }}/target/release/oxipng" "{{ flaca_dir }}/oxipng"
	@sudo chmod 755 "{{ flaca_dir }}/oxipng"

	just _info "Post-install cleaning."
	just _remove "{{ working_dir }}"

	just _success "install-oxipng"



# Download and install pngout (Linux only).
@install-pngout:
	just _header "install-oxipng"

	just _info "Checking dependencies."
	just _require "wget"
	@if [ {{ os() }} != "linux" ]; then just _error "Pngout has to be installed manually on your platform." && just _info "See: http://www.jonof.id.au/kenutils" && exit 1; fi

	just _info "Pre-install cleaning."
	just _remove "{{ flaca_dir }}/pngout"
	just _make_flaca_dir

	just _info "Downloading pngout."
	@wget -O /tmp/pngout {{ pngout_url }}
	@chmod 755 /tmp/pngout
	@sudo mv /tmp/pngout "{{ flaca_dir }}/pngout"

	just _success "install-pngout"



# Build and install Zopflipng.
@install-zopflipng:
	just _header "install-zopflipng"

	just _info "Checking dependencies."
	just _require "git"
	just _require "make"

	just _info "Pre-install cleaning."
	just _remove "{{ flaca_dir }}/zopflipng"
	just _make_flaca_dir

	just _info "Fetching source."
	just _fetch {{ zopflipng_url }}

	just _info "Building Zopflipng."
	@cd "{{ working_dir }}"; make zopflipng

	@sudo cp "{{ working_dir }}/zopflipng" "{{ flaca_dir }}/zopflipng"
	@sudo chmod 755 "{{ flaca_dir }}/zopflipng"

	just _info "Post-install cleaning."
	just _remove "{{ working_dir }}"

	just _success "install-zopflipng"



# Task header.
@_header TASK:
	echo "\e[34;1m[Task] \e[0;1m{{ TASK }}\e[0m"

# Echo an informational comment.
@_info COMMENT:
	echo "\e[95;1m[Info] \e[0;1m{{ COMMENT }}\e[0m"

# Echo an error.
@_error COMMENT:
	echo "\e[31;1m[Error] \e[0;1m{{ COMMENT }}\e[0m"

# Echo a success.
@_success COMMENT:
	echo "\e[92;1m[Success] \e[0;1m{{ COMMENT }} finished at `date`\e[0m"
	echo ""

# Fetch sources.
@_fetch REPO:
	just _info "Fetching sources."
	just _remove "{{ working_dir }}"
	@git clone "{{ REPO }}" "{{ working_dir }}"

# Make Flaca bin directory.
_make_flaca_dir:
	[ -e "{{ flaca_dir }}" ] || sudo mkdir -p "{{ flaca_dir }}"

# Remove a file or directory if it exists.
_remove FILE:
	[ ! -e "{{ FILE }}" ] || sudo rm -rf "{{ FILE }}"

# Replace strings within a file.
_replace FILE FROM TO:
	[ -f {{ FILE }} ] && sudo sed -i "s#{{ FROM }}#{{ TO }}#g" {{ FILE }}

# Require a dependency.
@_require THING:
	if [ ! $( command -v {{ THING }} ) ]; then just _error "Missing build dep '{{ THING }}'." && exit 1; fi

#!/bin/bash
#
# avm CLI binary installation script
# ------------------------------------
#
# Shamelessly borrowed from Metaboss CLI: [https://github.com/samuelvanderwaal/metaboss/blob/35ce0912/scripts/install.sh]
#
# The purpose of this script is to automate the download and installation
# of the avm binary.
#
# The script does a (simple) platform detection, downloads the binary
# for the detected platform and copies it to a folder in the PATH variable.
#
# Currently the supported platforms are Windows, macOS, Linux, or another Unix-like OS
# running variations of the sh Unix shells.
#

# 1. check latest avm version
# 2. if avm exists on path, check version
# 2.1  if installed version == latest version, exit
# 2.2  else ask user if they want to replace it
# 2.2.1  no -> exit
# 3. otherwise, save to ~/.avm/bin/avm
# 4. add ~/.avm/bin/avm to path if it does not exist

set -e

RED() { echo $'\e[1;31m'"$1"$'\e[0m'; }
GRN() { echo $'\e[1;32m'"$1"$'\e[0m'; }
CYN() { echo $'\e[1;36m'"$1"$'\e[0m'; }

begins_with() {
    case "$1" in
        "$2"*) true;;
        *) false;;
    esac
}

CYN  "avm cli binary installation script"
echo "---------------------------------------"
echo ""

OS_FLAVOUR="$(uname -s)"
PROCESSOR="$(uname -m)"

# we need to check whether we are running on an ARM
# architecture or not

case "${PROCESSOR}" in
    arm* | aarch* | ppc* )
        if [ "${OS_FLAVOUR}" != Darwin ]; then
            echo "Binary for ${PROCESSOR} architecture is not currently supported. Please follow the instructions at:"
            echo "  => $(CYN https://www.anchor-lang.com/docs/installation#install-anchor-cli)"
            echo ""
            echo "to build avm from the source code."
            exit 1
        fi
        ;;

    *)
        # good to go
        ;;
esac

REPO="solana-foundation/anchor"
RELEASE="latest"
BIN="avm"

echo "Checking for latest avm version"
LATEST_VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep "tag_name" | grep -Po 'v[^"]+')
echo "Latest avm version is $(CYN "${LATEST_VERSION}")"
echo ""

INSTALL_LOCATION="${HOME}/.avm/bin"

# binary already found on system, ask if we should replace it
if [ "$(command -v ${BIN})" != "" ]; then
    EXISTING="$(which "${BIN}")"
    EXISTING_VERSION="$("${EXISTING}" --version | cut -d " " -f 2)"

    echo "avm binary was found at:"
    echo "  => $(CYN "${EXISTING}")"
    echo "  => existing version $(CYN v"${EXISTING_VERSION}")"
    echo ""

    if [ "${EXISTING_VERSION}" = "${LATEST_VERSION#v}" ]; then
        GRN "You already have the latest version!"
        read -p "Do you want to continue (y/N)? " answer
        if [ "$answer" = "${answer#[Yy]}" ]; then
            exit 0
        fi
    fi

    read -p "$(CYN "Replace it? [y/N]") (default 'n'): " REPLACE
    if [ -z "${REPLACE}" ]; then
        REPLACE="n"
    fi

    # nothing else to do, replacement was cancelled
    if [ "${REPLACE}" = "${REPLACE#[Yy]}" ]; then
        RED "Aborting: replacement cancelled"
        exit 1
    fi

    INSTALL_LOCATION="$(dirname "${EXISTING}")"
    echo "'${BIN}' binary will be moved to '${INSTALL_LOCATION}'"
else
    echo "'${BIN}' binary will be installed at '${INSTALL_LOCATION}'"
fi
echo ""

PLATFORM=""
EXTENSION=""
if begins_with "${OS:-}" Windows; then
    PLATFORM="pc-windows-msvc"
    EXTENSION=".exe"
elif [ "${OS_FLAVOUR}" = Darwin ]; then
    PLATFORM="apple-darwin"
else
    PLATFORM="unknown-linux-gnu"
fi

echo "detected platform: $(CYN "${PLATFORM}")"
echo

RELEASE_URL="https://github.com/${REPO}/releases"
RELEASE="${LATEST_VERSION}"
read -p "Do you want the latest release (${LATEST_VERSION}) (Y/n): " answer
if [ "${answer#[Yy]}" = "" ] ;then
    echo "";
else
    echo "You can find all the releases here ${RELEASE_URL}"
    read -p "Enter release version (e.g, v0.8.7, v0.8.6): " RELEASE
fi

# creates a temporary directory to save the distribution file
SOURCE="$(mktemp -d)"

echo "$(CYN "1.") 🖥 $(CYN "Downloading distribution")"
echo ""

# downloads the distribution file
URL="${RELEASE_URL}/download/${RELEASE}/${BIN}-${RELEASE}${EXTENSION}"
DIST="${BIN}${EXTENSION}"
echo "Remote URL: ${URL}"
echo ""
curl -f -L "${URL}" --output "${SOURCE}/${DIST}"

SIZE=$(wc -c "${SOURCE}/${DIST}" | grep -oE "[0-9]+" | head -n 1)

if [ "${SIZE}" -eq 0 ]; then
    RED "Aborting: could not download avm distribution"
    exit 1
fi

# makes sure the binary will be executable
chmod u+x "${SOURCE}/${DIST}"

echo ""
echo "$(CYN "2.") 📤 $(CYN "Moving binary into place")"
echo ""

mv "${SOURCE}/${DIST}" "${INSTALL_LOCATION}/${DIST}"

# add to path if needed
if [ "$(command -v ${BIN})" = "" ]; then
    if begins_with "${OS:-}" Windows; then
        echo "  => adding '${INSTALL_DIR}' to 'PATH' variable in '$ENV_FILE'"
        cmd "/c setx PATH=%PATH%;$(cygpath -w "${INSTALL_LOCATION}")"
    else
        ENV_FILE="${HOME}/.$(basename "$SHELL")rc"

        if [ -f "$ENV_FILE" ]; then
            echo "  => adding '${INSTALL_DIR}' to 'PATH' variable in '$ENV_FILE'"
            echo "export PATH=\"$HOME/bin:\$PATH\"" >> "$ENV_FILE"
        else
            echo "  => adding '${INSTALL_DIR}' to 'PATH' variable to execute 'avm' from any directory."
            echo "     - file '$(CYN "$ENV_FILE")' was not found"
            echo ""
            read -p "$(CYN "Would you like to create '$ENV_FILE'? [Y/n]") (default 'n'): " CREATE

            if [ -z "${REPLACE}" ]; then
                CREATE="n"
            fi

            if [ "$CREATE" != "${CREATE#[Yy]}" ]; then
                echo "  => adding '${INSTALL_DIR}' to 'PATH' variable in '$ENV_FILE'"
                echo "export PATH=\"$HOME/bin:\$PATH\"" >> "$ENV_FILE"
            else
                echo ""
                echo "     $(RED "[File creation cancelled]")"
                echo ""
                echo "     - to manually add '${INSTALL_DIR}' to 'PATH' you will need to:"
                echo ""
                echo "       1. create a file named '$(basename "$ENV_FILE")' in your directory '$(dirname "$ENV_FILE")'"
                echo "       2. add the following line to the file:"
                echo ""
                echo "           export PATH=\"$HOME/bin:\$PATH\""
            fi
        fi
    fi
fi

echo ""
# sanity check
if [ "$(command -v $BIN)" = "" ]; then
    # installation was completed, but avm is not in the PATH
    echo "✅ $(GRN "Installation complete:") restart your shell to update 'PATH' variable or type '${INSTALL_DIR}/${BIN}' to start using it."
else
    # success
    echo "✅ $(GRN "Installation successful:") type '${BIN}' to start using it."
fi

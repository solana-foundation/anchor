#!/bin/bash
#
# avm CLI binary installation script
# ------------------------------------
#
# Inspired from: [https://github.com/samuelvanderwaal/metaboss/blob/35ce0912/scripts/install.sh]
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

OS_FLAVOUR="$(uname -s)"
PROCESSOR="$(uname -m)"

handle_exit() {
    errorcode="$?"

    # if called explicitly, then exit cleanly
    if [ -n "$1" ]; then
        exit "$1"
    fi

    CREATE_ISSUE_LINK="https://github.com/solana-foundation/anchor/issues/new?title=%5Binstaller%5D%20Error%20in%20install%20script%3A%20&description=%3C%21--%20debug%20information%20--%3E%0A%60%60%60text%0AShell%3A%20%0ACurl%3A%20%0AOS%3A%20%0AProcessor%3A%20%0A%60%60%60%0A%0A%23%23%20Issue%20Details%0A%3C%21--%20please%20describe%20what%20you%20were%20trying%20to%20do%2C%20and%20any%20relevant%20details%20--%3E%0A%0A%23%23%20Script%20Output%0A%3C%21--%20paste%20the%20output%20of%20the%20install%20script%20between%20the%20%60%60%60%20%20--%3E%0A%60%60%60console%0A%0A%60%60%60"

    # else, something went wrong
    echo ""
    echo ""
    RED "Error: install script exited with error code $(CYN "${errorcode}")"
    echo ""
    echo "Please check the docs for troubleshooting tips and alternative installation methods"
    echo "    => $(CYN https://www.anchor-lang.com/docs/installation#install-anchor-cli)"
    echo ""
    echo "We would also greatly appreciate a bug report at:"
    echo "    => $(CYN "${CREATE_ISSUE_LINK}")"
    echo ""
    echo "Please include the complete output of the script, including the following debug information:"
    echo ""
    CYN "Shell:"
    CYN "$("${SHELL}" --version || echo "could not get shell version: ${SHELL} ($?)")"
    CYN ""
    CYN "Curl:"
    CYN "$(curl --version || echo "could not get curl version: ($?)")"
    CYN ""
    CYN "OS: ${OS_FLAVOUR}"
    CYN "Processor: ${PROCESSOR}"
}

trap 'handle_exit' ERR

CYN  "avm cli binary installation script"
echo "---------------------------------------"
echo ""

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
        echo ""
        read -r -p "$(CYN "Do you want to continue? [y/N]") (default 'n'): " answer
        if [ "$answer" = "${answer#[Yy]}" ]; then
            exit 0
        fi
    fi

    read -r -p "$(CYN "Replace it? [y/N]"): " REPLACE
    if [ -z "${REPLACE}" ]; then
        REPLACE="n"
    fi

    # nothing else to do, replacement was cancelled
    if [ "${REPLACE}" = "${REPLACE#[Yy]}" ]; then
        RED "Aborting: replacement cancelled"
        handle_exit 1
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
    PLATFORM="x86_64-pc-windows-msvc"
    EXTENSION=".exe"
elif [ "${OS_FLAVOUR}" = Darwin ]; then
    case "${PROCESSOR}" in
        arm* )
            PLATFORM="aarch64-apple-darwin"
            ;;
        *)
            PLATFORM="x86_64-apple-darwin"
            ;;
    esac
else
    PLATFORM="x86_64-unknown-linux-gnu"
fi

echo "detected platform: $(CYN "${PLATFORM}")"

RELEASE_URL="https://github.com/${REPO}/releases"
RELEASE="${LATEST_VERSION}"
read -r -p "Do you want the latest release (${LATEST_VERSION})? [Y/n]: " answer
if [ "${answer#[Yy]}" = "" ] ;then
    echo ""
else
    echo ""
    echo "You can find all the releases here ${RELEASE_URL}"
    read -r -p "Enter release version (e.g, v0.8.7, v0.8.6): " RELEASE
    echo ""
fi

# creates a temporary directory to save the distribution file
SOURCE="$(mktemp -d)"

echo "$(CYN "1.") 🖥 $(CYN "Downloading distribution")"
echo ""

# downloads the distribution file
URL="${RELEASE_URL}/download/${RELEASE}/${BIN}-${RELEASE#v}-${PLATFORM}${EXTENSION}"
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

mkdir -p "${INSTALL_LOCATION}"
mv "${SOURCE}/${DIST}" "${INSTALL_LOCATION}/${DIST}"

# add to path if needed
if [ "$(command -v ${BIN})" = "" ]; then
    if begins_with "${OS:-}" Windows; then
        echo "  => adding '${INSTALL_LOCATION}' to 'PATH'"
        cmd "/c setx PATH=%PATH%;$(cygpath -w "${INSTALL_LOCATION}")"
    else
        ENV_FILE="${HOME}/.$(basename "$SHELL")rc"

        if [ -f "${ENV_FILE}" ]; then
            echo "  => adding '${INSTALL_LOCATION}' to 'PATH' variable in '${ENV_FILE}'"
            echo "export PATH=\"${INSTALL_LOCATION}:\${PATH}\"" >> "${ENV_FILE}"
        else
            echo "  => adding '${INSTALL_LOCATION}' to 'PATH' variable to execute 'avm' from any directory."
            echo "     - file '$(CYN "${ENV_FILE}")' was not found"
            echo ""
            read -r -p "$(CYN "Would you like to create '${ENV_FILE}'? [Y/n]") (default 'n'): " CREATE

            if [ -z "${REPLACE}" ]; then
                CREATE="n"
            fi

            if [ "$CREATE" != "${CREATE#[Yy]}" ]; then
                echo "  => adding '${INSTALL_LOCATION}' to 'PATH' variable in '${ENV_FILE}'"
                echo "export PATH=\"${INSTALL_LOCATION}:\${PATH}\"" >> "${ENV_FILE}"
            else
                echo ""
                echo "     $(RED "[File creation cancelled]")"
                echo ""
                echo "     - to manually add '${INSTALL_LOCATION}' to 'PATH' you will need to:"
                echo ""
                echo "       1. create a file named '$(basename "${ENV_FILE}")' in your directory '$(dirname "${ENV_FILE}")'"
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
    echo "✅ $(GRN "Installation complete:") restart your shell to update 'PATH' variable"
    echo "    or, type '${INSTALL_LOCATION}/${BIN}' to start using it."
else
    # success
    echo "✅ $(GRN "Installation successful:") type '${BIN}' to start using it."
fi

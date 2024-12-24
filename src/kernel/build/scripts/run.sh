# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Script Arguments
TARGET=$1   # Target
MACHINE=$2  # Machine
IMAGE=$3    # Image
MODE=$4     # Run Mode
TIMEOUT=$5  # Timeout

# Global Variables
export SCRIPT_NAME=$0
export SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd)"

#===================================================================================================
# usage()
#===================================================================================================

#
# Prints script usage and exits.
#
function usage
{
	echo "$SCRIPT_NAME <target> <machine> <image> [mode] [timeout]"
	exit 1
}

#===================================================================================================
# check_args()
#===================================================================================================

# Check script arguments.
function check_args
{
	# Missing binary?
	if [ -z $IMAGE ];
	then
		echo "$SCRIPT_NAME: missing image"
		usage
	fi
}

#===================================================================================================
# run_qemu()
#===================================================================================================

# Runs a binary in QEMU.
function run_qemu
{
	local target=$1     # Target architecture.
	local machine=$2    # Machine.
	local image=$3      # Image.
	local mode=$4       # Spawn mode (run or debug).
	local timeout=$5    # Timeout for test mode.
	local GDB_PORT=1234 # GDB port used for debugging.
	local cmd=""

	# Target configuration.
	local MEMSIZE=256M # Memory Size

	# Check if the target is unsupported.
	if [ $target != "i386" ]; then
		echo "Unsupported target: $target"
		exit 1
	fi

	case "$machine" in
		"qemu-baremetal")
			machine="-machine pc"
			stdout="-serial stdio"
			smp=""
			;;
		"qemu-baremetal-smp")
			machine="-machine pc"
			stdout="-serial stdio"
			smp="-smp 2"
			;;
		"qemu-pc")
			machine="-machine pc"
			stdout="-debugcon stdio"
			smp=""
			;;
		"qemu-pc-smp")
			machine="-machine pc"
			stdout="-debugcon stdio"
			smp="-smp 2"
			;;
		"qemu-isapc")
			machine="-machine isapc"
			stdout="-debugcon stdio"
			smp=""
			;;
		*)
			echo "Unsupported machine: $MACHINE"
			exit 1
			;;
	esac

	# Select QEMU from path, if available.
	if [ ! -z $(command -v qemu-system-$target) ];
	then
		qemu_cmd="qemu-system-$target"
	else
		qemu_cmd="$TOOLCHAIN_DIR/qemu/bin/qemu-system-$target"
	fi

	qemu_cmd="$qemu_cmd
	  		$machine
			$stdout
			$smp
			-display none
			-cpu pentium2
			-m $MEMSIZE
			-mem-prealloc"

	cmd="$qemu_cmd -cdrom $image"

	# Run.
	if [ $mode == "--debug" ];
	then
		cmd="$cmd -gdb tcp::$GDB_PORT -S"
		$cmd
	else

		if [ ! -z $timeout ];
		then
			cmd="timeout -s SIGINT --preserve-status --foreground $timeout $cmd"
		fi

		$cmd
	fi
}

#===================================================================================================

# Runs a binary in MicroVm.
function run_microvm()
{
	local image=$1   # Image.
	local timeout=$2 # Timeout for test mode.

	# Base command.
	local cmd="$MICROVM_PATH/microvm.elf"

	# Machine configuration.
	local MEMSIZE=256M # Memory Size

	cmd="$cmd -kernel $image -memory $MEMSIZE"

	# Run.
	if [ ! -z $timeout ];
	then
		cmd="timeout -s SIGINT --preserve-status --foreground $timeout sudo -E $cmd"
	fi

	echo "Running: $cmd"

	$cmd
}

#===================================================================================================

# No debug mode.
if [ -z $MODE ];
then
	MODE="--no-debug"
fi

# Verbose mode.
if [[ ! -z $VERBOSE ]];
then
	echo "====================================================================="
	echo "TARGET      = $TARGET"
	echo "MACHINE     = $MACHINE"
	echo "SCRIPT_DIR  = $SCRIPT_DIR"
	echo "SCRIPT_NAME = $SCRIPT_NAME"
	echo "IMAGE       = $IMAGE"
	echo "MODE        = $MODE"
	echo "TIMEOUT     = $TIMEOUT"
	echo "====================================================================="
fi

case "$TARGET" in
	"x86")
		check_args
		case "$MACHINE" in
			"qemu-baremetal" | "qemu-baremetal-smp" | "qemu-pc" | "qemu-pc-smp" | "qemu-isapc")
				run_qemu "i386" $MACHINE $IMAGE $MODE $TIMEOUT
				;;
			"microvm")
				run_microvm $IMAGE $TIMEOUT
				;;
			*)
				echo "Unsupported machine: $MACHINE"
				;;
		esac
		;;
    *)
        echo "Unsupported target: $TARGET"
        ;;
esac

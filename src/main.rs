// Copyright (c) 2014 Jared Stafford (jspenguin@jspenguin.org)
// Copyright (c) 2024 Damir Jelić
// Copyright (c) 2024 Lukas Lichten
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{
    fs::{remove_file, File},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Ok, Result};
use clap::Parser;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_TEMPORARY;

use crate::file_mapping::FileMapping;

mod file_mapping;

const LONG_ABOUT: &str = "Shared Memory Bridge facilitates sharing memory between Windows\n\
                          applications running under Wine/Proton and Linux, offering a seamless\n\
                          way to access and manipulate named shared memory spaces across these\n\
                          platforms. It's particularly useful in gaming and simulations, allowing\n\
                          Linux applications to directly read data from Windows applications.\n\n\
                          Example Usage:\n\n\
                          To launch the bridge and view command line options, use the following \
                          command:\n    \
                              protontricks-launch --appid APPID shm-bridge.exe\n\n\
                          This will display help output and available options for `shm-bridge`,\n\
                          guiding you through the necessary steps to set up and run the bridge\n\
                          within your specific environment.";

#[derive(Parser)]
#[command(author, version, about, long_about = LONG_ABOUT)]
struct Cli {
    #[arg(short, long, num_args(1..), help = "name of the shared memory map (can pass multiple to define multiple maps)")]
    map: Vec<String>,

    #[arg(short, long, num_args(1..), help = "size of the shared memory map (has to be the same number as map arguments)")]
    size: Vec<usize>,

    #[arg(long, help = "doesn't launch the bridge, instead cleans up /dev/shm from these maps (in case of hard termination of the bridge) and exits")]
    clean_up: bool
}

fn find_shm_dir() -> PathBuf {
    // TODO: Support non-standard tmpfs mount points. This can be achieved by
    // parsing `/proc/mounts`, or if that's not available, by parsing `/etc/fstab`.

    /// The default path for our tmpfs.
    const TMPFS_PATH: &str = "/dev/shm/";

    // TODO: We should also check that /dev/shm, or any other filesystem we found
    // using `/proc/mounts` is actually a `tmpfs`. This is sadly problematic, I
    // tried to use `GetVolumeInformationW` but, as the name suggest, it expects
    // a volume, so `C:\\`, or as Wine exposes `/`, `Z:\\`. We can't check the
    // file system name of `Z:\\dev\shm` for example. Even if we do check the
    // filesystem name of `Z:\\` we get `NTFS` back.

    PathBuf::from(TMPFS_PATH)
}

fn create_file_mapping(dir: &Path, file_name: &str, size: usize) -> Result<FileMapping> {
    let path = dir.join(file_name);

    // First we create a /dev/shm backed file.
    //
    // Now hear me out, usually we should use `shm_open(3)` here, but on Linux
    // `shm_open()` just calls `open()`. It does have some logic to find the
    // tmpfs location if it's mounted in a non-standard location. Since we can't
    // call `shm_open(3)` from inside the Wine environment
    let file = File::options()
        .read(true)
        .write(true)
        .attributes(FILE_ATTRIBUTE_TEMPORARY.0)
        .create(true)
        .open(&path)
        .context(format!("Could not open the tmpfs file: {path:?}"))?;

    // Now we create a mapping that is backed by the previously created /dev/shm`
    // file.
    let mapping = FileMapping::new(
        // We're going to use the same names the Simulator uses. This ensures that the
        // simulator will reuse this `/dev/shm` backed mapping instead of creating a new anonymous
        // one. Making the simulator reuse the mapping in turn means that the telemetry data will
        // be available in `/dev/shm` as well, making it accessible to Linux.
        file_name,
        // Pass in the handle of the `/dev/shm` file, this ensures that the file mapping is a file
        // backed one and is using our tmpfs file created on the Linux side.
        &file,
        // The documentation[1] for CreateFileMapping states that the sizes are only necessary if
        // we're using a `INVALID_HANDLE_VALUE` for the file handle.
        //
        // It also states the following:
        // > If this parameter and dwMaximumSizeHigh are 0 (zero), the maximum size of the
        // > file mapping object is equal to the current size of the file that hFile identifies.
        //
        // This sadly doesn't seem to work with our `/dev/shm` file and makes the Simulator crash,
        // so we're passing the sizes manually.
        //
        // [1]: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createfilemappinga#parameters
        size,
    )?;

    // Return the mapping, the caller needs to ensure that the mapping object stays
    // alive. On the other hand, the `/dev/shm` backed file can be closed.
    Ok(mapping)
}

fn main() -> Result<()> {
    let args = Cli::parse();
    if args.size.len() != args.map.len() && !args.clean_up {
        println!("Error: Incorrect Argument count, --map has to have the same as --size (found {}:{})", args.map.len(), args.size.len());
        println!("Exiting...");
        std::process::exit(1); // Could we pass Err? Maybe, but this is good enough right now
    }
    if args.map.is_empty() {
        println!("Error: Require at least one --map (with --size) to be defined!");
        println!("Exiting...");
        std::process::exit(1);
    }

    // Find a suitable tmpfs based mountpoint, this is usually `/dev/shm`.
    let shm_dir = find_shm_dir();

    // Handles clean up, where we skip mounting the memory maps
    if args.clean_up {
        return clean_up(&args, shm_dir);
    }

    let mut mappings = Vec::new();


    println!("Found a tmpfs filesystem at {}", shm_dir.to_string_lossy());

    for (file_name, size) in args.map.iter().zip(args.size.iter()) {
        let mapping = create_file_mapping(&shm_dir, file_name, *size)
            .with_context(|| format!("Error creating a file mapping for {file_name}"))?;

        println!("Created a tmpfs backed mapping for {file_name} with size {size}");
        mappings.push(mapping);
    }

    let current_thread = std::thread::current();

    // Set a CTRL_C_EVENT/CTRL_BREAK_EVENT handler which will unpark our thread and
    // let main finish.
    ctrlc::set_handler(move || {
        current_thread.unpark();
    })
    .expect("We should be able to set up a CTRL-C handler.");

    println!("All mappings were successfully created, press CTRL-C to exit.");

    // Park the main thread so we don't exit and don't drop the `FileMapping`
    // objects.
    std::thread::park();

    println!("\nShutting down.");

    // The CTRL-C handler has unparked us, somebody wants us to stop running so
    // let's unlink the `/dev/shm` files.
    clean_up(&args, shm_dir)?;

    Ok(())
}

/// This is a sperate function to allow calling later clean up
/// when the original process is terminated without getting to finish
/// (sigkill for example)
fn clean_up(args: &Cli, shm_dir: PathBuf) -> Result<()> {
    for file_name in args.map.iter() {
        println!("Removing mapping {file_name}");
        let path = shm_dir.join(file_name);

        if !path.exists() {
            println!("Failed to unlink /dev/shm/{file_name} as it does not exist");
        } else {
            remove_file(&path)
                .with_context(|| format!("Could not unlink the /dev/shm backed file {file_name}"))?;
        }
    }

    Ok(())
}

# Node-based cloud video editing service - Documentation
This brief documentation outlines how the code is structured, how to get up and running with the code, and the known bugs with the code.

## Code Structure
The code is split into three main directories: `client`, `server` and `shared`.

### The `shared` directory
The `shared` directory contains code which is utilised by both the client and server side of the application. This includes all the 'Main Functionality' described in the main report.

The `src/nodes` directory contains a file for each node type that exists. At the moment, only 5 node types are supported: media import node; blur node; concatenation node; output node; volume node. These are then all utilised in the `src/nodes/mod.rs` which puts each node type into a `HashMap` - this becomes the Node Register for the application.

The `src/networking.rs` file contains utility functions for handling networking between the client and the server, as well some networking constants, for example the port that the server will be run at, and the client will connect to.

The `src/constants.rs` file contains utility functions for both the server and the client to be able to obtain certain file paths (e.g. where to save media files) easily, with a static function.

The `src/cache.rs`, `src/clip.rs`, `src/node.rs`, `src/pipeline.rs` and `src/store.rs` files contain the main functionality described in the report.  

This directory cannot be executed, since it is only a library for both the client and server to utilise in their separate Rust projects.
### The `server` directory
The `server` directory contains code utilised by only the server for its functions. 

The `main.rs` is the bulk of the code, which handles the TCP server with a `Threadpool`. Once it receives a new connection, it expects an initial message indicating the connection's intent for that connection. It will then follow the protocol for that particular task. Some tasks are moved into separate functions for better readability.

One implementation detail to note is the `generate_pipeline_in_process` function; this will try up to 10 times to generate the pipeline on a separate process; this is simply due to the fact that the GES operations are unsafe, and can cause segmentation faults otherwise - hence we put them on a separate process, and we try it multiple times in case of segmentation faults. It is not a clean solution, but it solves the issue of segmentation faults in the majority of cases.

The `gst_process.rs` is the file which contains the logic for each process in the process pool. This process pool is used to execute the pipelines generated in the previous stage, and notify the parent process when specific chunks are complete.

### The `client` directory
The `client` directory is split into two main directories: `src-tauri` and `src`.

`src-tauri` contains the logic for the Rust backend of the client, which includes the networking for the client with the server, the handling of tasks via the task manager, and the management of video preview generation data. 

`src` contains the user interface code, which is split up into `classes`, which contains some utility functions, as well as the `EventBus` and `Cache` described in the report, and the `components`, which features a file/directory per component of the UI. 

## Installation/Setting up the environment
The runtime environment for the application is unfortunately quite environment dependent. I have modified some of the code to be as compatible as possible - for example, I have removed hardware accelerated video encoding on the server, since that is GPU-dependent (this has been replaced with the standard `x264enc` software encoder).

### The Server
The server cannot be run on Windows. My development environment uses Windows Subsystem for Linux (WSL) with Ubuntu 21.10. For Ubuntu systems, the following packages will need to be installed before proceeding (via `apt-get` for Ubuntu): 
`libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-x gstreamer1.0-alsa gstreamer1.0-gl gstreamer1.0-gtk3 gstreamer1.0-qt5 gstreamer1.0-pulseaudio libglib2.0-dev libcairo2-dev libjpeg-dev libges-1.0-dev librust-pango-sys-dev libgtk-3-dev libssl-dev build-essential pkg-config`

Note: these packages require Ubuntu 21.10, and will not allow the code to be correctly compiled (at least by default) on Ubuntu 20.04, for example.

Once these packages have been installed, Rust then needs to be installed on the server. 

Finally, the server can be run by following the commands:
- `cd server`
- `cargo run`
This should then compile and run the server, which by default will be running on port 3001.
### The Client

Firstly, you'll need Rust and `npm` installed. Then, you will need to obtain the IP address of the server, and place it in the `SERVER_HOST` environment variable in `client/.env`. If this environment variable is not specified, the client will assume the server is running at localhost (`127.0.0.1`). 

Then, you will want to follow the following steps:
- `cd client`
- `npm i`
- `npm run dev`
This should install all the relevant packages, and start up the React development server, as well as compile and run the Tauri application, which should open the client application.

## Known Issues and Bugs
### Server
Primarily, the server may experience segmentation faults. I have put in some safeguards to mitigate these, but unfortunately I have still experienced occasional errors which I have yet to find the root cause of.
### Client
The main issue with the client in its current state is the video preview; as documented in the report, MSE can be incredibly difficult to use from a developer standpoint, due to its unusual error reporting strategy. As a result, video previews may not appear as expected.

Furthermore, the interface for the video preview is somewhat unfinished, since most of the development time was taken focused on fixing the issues that MSE brought. 
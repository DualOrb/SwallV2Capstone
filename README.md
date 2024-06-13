# sWall Project

## Dev Setup

Steps:
1. Clone the repo onto the host machine
2. Create the dev environement container (see below)
3. Clone the repo into docker container
4. Open the container w/ VsCode "Dev Container" extension.
5. Tips and Tricks (see below)

### Create the Dev Environment

```bash
# Build + start the dev container
docker compose up -d --build

# Connect to the dev container
docker exec -ti swall-dev-container /bin/bash
```

You can attach a VsCode instance to the container with the "Dev Container" extension.

_Note: You can see the container if you open Docker Desktop._

_Warnings: Everything outside `/root/persistent` may be wiped at anytime if you destroy the container._

## Building Compositor
The compositor needs to link with various dependencies. `./Dockerfile` is the environment it should be built in (see Dev Setup).

```bash
cd /root/persistence/swall-project

# You will need to change directory to whatever crate you want to build first.
cargo build
```

## Tips and Tricks
### Accessing Container Files
On windows you can access `/root/persitent` in file explorer by going to `\\wsl.localhost\docker-desktop-data\data\docker\volumes\swall-project_swall-project-working\_data`.

This is nice for opening the rust documentation.

_Warning: Don't open VsCode on `\\wsl.local\...` directly. Use "Dev Container" to connect to docker!_
_Warning: Running arbitrary commands from the host machine inside `\\wsl.local\...` might cause funkiness._

### Auto-complete and other Development Tools
Developer tools can be installed into the container by running:
```bash
~/comfort && bash # Installs neovim and enables autocomplete
```

### `XDG_RUNTIME_DIR` not defined / no compositor found
This is the directory where the compositor and wayland clients will look for the wayland socket.
Just define an environment variable that points somewhere.
_This should be solved in newer versions of the docker container._

## Building Client
The raspberry pi should have docker already installed

In the project root, run 
```docker compose -f client-docker-compose.yaml up -d --build```

The container will compile the project into a .so file, make a clean target container, copy the .so file over and run the pipeline as its only command. env variable is set so gstreamer knows where to find the custom plugins

Ensure the docker service is running on the pi, so that docker will launch on startup and automatically run our container
```systemctl enable docker```
from swall_socket import SwallSocket

class SwallApi:
    """Class for communicating with the Swall's Custom Wayland Compositor"""

    def __init__(self) -> None:
        """Initialize the SwallApi object and establish connection."""
        self.swall_socket = SwallSocket()

    def spawn(self, coordinate_x, coordinate_y, width, height, command) -> object:
        """Spawn a new process in the compositor."""

        # Format into json
        json_object = {
            "Spawn": {
                "config": {
                    "executable": command[0],
                    "args": command[1:],
                    "area": {
                        "x": coordinate_x,
                        "y": coordinate_y,
                        "width": width,
                        "height": height,
                    },
                }
            }
        }

        # Send to compositor
        self.swall_socket.send(json_object)

        # Process response
        return self.swall_socket.receive()

    def kill(self, pid) -> str:
        """Kill a process in the compositor"""

        # Format into json
        json_object = {"Kill": {"pid": pid}}

        self.swall_socket.send(json_object)

        return self.swall_socket.receive()

    def list(self) -> str:
        """List all processes currently running in the compositor"""
        self.swall_socket.send("List")

        return self.swall_socket.receive()

    def screen_size(self) -> str:
        """Get the screen size from the compositor"""
        self.swall_socket.send("ScreenSize")

        return self.swall_socket.receive()

    def resize(self, pid, coordinate_x, coordinate_y, width, height) -> str:
        """Resize a process in the compositor"""

        json_object = {
            "Move": {
                "pid": pid,
                "rect": {
                    "x": coordinate_x,
                    "y": coordinate_y,
                    "height": height,
                    "width": width,
                },
            }
        }

        # Send to compositor
        self.swall_socket.send(json_object)

        # Process response
        return self.swall_socket.receive()

    

from socket import *
import json
from time import sleep

class SwallSocket:
    """Manages the state and communication with the Swall Socket through composition"""

    MSG_SPLITTER = b"\x1e"
    SOCKET_PATH = "/tmp/swall/control-0"

    def __init__(self) -> None:
        self.residual_data = b""
        self.start_connection()

    def send(self, json_object: object):
        """Send a message to the compositor"""
        message = json.dumps(json_object)

        self.swall_socket.send(message.encode() + SwallSocket.MSG_SPLITTER)

    def receive(self) -> object:
        """Receive a message from the compositor"""
        # Receive data until a single message can be split from the stream
        while True:
            possible_new_msg, has_splitter, self.residual_data = self.residual_data.partition(self.MSG_SPLITTER)
            if has_splitter:
                return json.loads(possible_new_msg.decode())

            self.residual_data = possible_new_msg
            self.residual_data += self.swall_socket.recv(1024)

    def close_connection(self):
        """Close the connection to the compositor"""
        self.swall_socket.close()

    def start_connection(self):
        """Start the connection to the compositor"""

        # Clear partial messages from internal buffer
        self.residual_data = b""

        while True:
            try:
                self.swall_socket = socket(AF_UNIX, SOCK_STREAM)
                self.swall_socket.connect(SwallSocket.SOCKET_PATH)
                return
            except timeout:
                print(
                    "Unable to initiate connection. Compositor may not be started. Trying again in 30 seconds"
                )

                sleep(30)
            except OSError as e:
                print(
                    "Unable to initiate connection: " + str(e)
                )

# A script for testing the connection to the app controller socket (Preliminary before state implementation)
# Other side should print out msg

from socket import *
import os
import json

socket_path = '/tmp/swall/control-0'

client_socket = socket(AF_UNIX, SOCK_STREAM)

client_socket.connect(socket_path)

o = json.dumps("List")

print("Sent Command: " + str(o))

client_socket.send(o.encode() + b'\x1e')

response = client_socket.recv(1024)
print(f'Received Response: {response.decode()}')

client_socket.close()
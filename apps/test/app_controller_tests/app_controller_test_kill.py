# A script for testing the connection to the app controller socket (Preliminary before state implementation)
# Other side should print out msg

from socket import *
import json
from ast import literal_eval
import os

# Retrieve the list of active processes on the compositor machine
socket_path = '/tmp/swall/control-0'

client_socket = socket(AF_UNIX, SOCK_STREAM)

client_socket.connect(socket_path)

o = json.dumps("List")

client_socket.send(o.encode() + b'\x1e')

print("Sent Command: " + str(o))
response = client_socket.recv(1024)
# print(f'Received Response: {response.decode()}')
print("Received Response: " + str(response.decode()))
client_socket.close()

all_processes = json.loads(response.decode())["process_ids"]

print(all_processes)

if len(all_processes) >= 1:
    # Kill the first process we find
    first_process = all_processes[0]

    # Fire the kill shot

    client_socket = socket(AF_UNIX, SOCK_STREAM)

    client_socket.connect(socket_path)


    k = json.dumps({ "Kill" : { "pid" : first_process } })

    print("Sent Command: " + str(k))

    client_socket.send(k.encode())

    response = client_socket.recv(1024)
    print(f'Received Response: {response.decode()}')

    client_socket.close()
else:
    print("No processes to kill")

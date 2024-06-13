# Main event loop and response handling loop
import swall_api
from bcolors import bcolors
import os
from prettytable import PrettyTable

def main():
    # Initial setup of cache and info
    controller = swall_api.SwallApi()
    TERMINAL_SIZE = os.get_terminal_size()

    print(
        "\n"
        + bcolors.BOLD
        + bcolors.FAIL
        + "CARLETON UNIVERSITY SWALL APP CONTROLLER"
        + bcolors.ENDC
        + "\n"
    )

    while True:

        print(
            "List of valid commands are ["
            + bcolors.BOLD
            + bcolors.OKBLUE
            + " SPAWN   KILL   LIST   SCREEN_SIZE   MOVE    HELP   QUIT"
            + bcolors.ENDC
            + "]"
        )

        choice = input("\nEnter a command: ").strip().lower()

        response = None

        match (choice):
            case "spawn":
                # Get user's input
                coordinate_x = get_user_int("Enter x coordinate")
                coordinate_y = get_user_int("Enter y coordinate")
                width = get_user_int("Enter width")
                height = get_user_int("Enter height")

                command = input("Enter the full exeuteable command: ").split(" ")
                response = controller.spawn(coordinate_x, coordinate_y, width, height, command)
                if response["error"] is None:
                    print(
                        "\nSpawned Application "
                        + str(response["config"]["executable"])
                        + " with PID "
                        + bcolors.OKCYAN
                        + bcolors.BOLD
                        + ""
                        + str(response["pid"])
                        + bcolors.ENDC
                        + "\n"
                    )
            case "kill":
                pid = get_user_int("Enter the pid")
                response = controller.kill(pid)
            case "list":
                response = controller.list()
                table = PrettyTable()
                table.field_names = [
                    "Executeable",
                    "PID",
                    "X Coord",
                    "Y Coord",
                    "Width",
                    "Height",
                    "Args",
                ]

                for tup in response["process_ids"]:
                    table.add_row(
                        [
                            bcolors.OKGREEN
                            + bcolors.BOLD
                            + str(tup[1]["executable"])
                            + " "
                            + bcolors.ENDC,
                            bcolors.OKCYAN + bcolors.BOLD + str(tup[0]) + bcolors.ENDC,
                            str(tup[1]["area"]["x"]),
                            str(tup[1]["area"]["y"]),
                            str(tup[1]["area"]["width"]),
                            str(tup[1]["area"]["height"]),
                            str(tup[1]["args"]),
                        ]
                    )
                print(table)
            case "screen_size":
                response = controller.screen_size()
                print(
                    "\nWIDTH "
                    + bcolors.OKCYAN
                    + bcolors.BOLD
                    + str(response["screen_width"])
                    + bcolors.ENDC
                    + " HEIGHT "
                    + bcolors.OKCYAN
                    + bcolors.BOLD
                    + str(response["screen_height"])
                    + bcolors.ENDC
                    + "\n"
                )
            case "move":
                print("NOTICE: Current functionality apps can only move. They do not resize")
                pid = get_user_int("Enter the pid")
                coordinate_x = get_user_int("Enter x coordinate")
                coordinate_y = get_user_int("Enter y coordinate")
                width = get_user_int("Enter width")
                height = get_user_int("Enter height")
                response = controller.resize(pid, coordinate_x, coordinate_y, width, height)
            case "quit":
                break
            case "help":
                print(
                    bcolors.OKCYAN
                    + bcolors.BOLD
                    + "SPAWN "
                    + bcolors.ENDC
                    + "- Spawns an application via their command line launch arguments\n"
                )
                print(
                    bcolors.OKCYAN
                    + bcolors.BOLD
                    + "KILL "
                    + bcolors.ENDC
                    + "- Kills an application given a specific process id\n"
                )
                print(
                    bcolors.OKCYAN
                    + bcolors.BOLD
                    + "LIST "
                    + bcolors.ENDC
                    + "- Lists all the currently running applications on the SWall with their process ids\n"
                )
                print(
                    bcolors.OKCYAN
                    + bcolors.BOLD
                    + "SCREEN_SIZE "
                    + bcolors.ENDC
                    + "- Returns the current screen size of the backend compositor\n"
                )
                print(
                    bcolors.OKCYAN
                    + bcolors.BOLD
                    + "MOVE "
                    + bcolors.ENDC
                    + "- Moves the window to a new position\n"
                )
            case _:
                print("Invalid command entered. Try again\n")

        if response is not None and response.get("error") is not None:
            print("Error Occured on action: " + response["error"])
        elif response is not None and response["success"] == True:
            print(bcolors.BOLD + bcolors.OKGREEN + "Success" + bcolors.ENDC + "\n")

        print("=" * TERMINAL_SIZE.columns)

def get_user_int(prompt: str) -> int:
        """Get an integer input from the user"""
        while True:
            try:
                return int(input(prompt + ": "))
            except ValueError as error:
                print("Invalid input")
                continue

if __name__ == "__main__":
    main()
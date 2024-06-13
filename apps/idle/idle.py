import pygame
import sys
from time import time as clock

def next_rect(width, height):
    width -= SQUARE_SIZE
    height -= SQUARE_SIZE
    time = clock()
    if ((SPEED * time // (width - SQUARE_SIZE))) % 2:
        next_x = SPEED * time % (width - SQUARE_SIZE)
    else:
        next_x = (width - SQUARE_SIZE) - (SPEED * time % (width - SQUARE_SIZE))
    if ((SPEED * time // (height - SQUARE_SIZE))) % 2:
        next_y = SPEED * time % (height - SQUARE_SIZE)
    else:
        next_y = (height - SQUARE_SIZE) - (SPEED * time % (height - SQUARE_SIZE))
    return pygame.Rect(next_x, next_y, SQUARE_SIZE, SQUARE_SIZE)

SPEED = 100
SQUARE_SIZE = 75;
BLACK = pygame.Color(0, 0, 0)
WHITE = pygame.Color(255, 255, 255)

def main():
    try:
        width, height = int(sys.argv[1]), int(sys.argv[2])
    except:
        width, height = 1440, 720

    pygame.init()

    screen = pygame.display.set_mode((width, height), pygame.RESIZABLE)

    ballrect = pygame.Rect(0, 0, SQUARE_SIZE, SQUARE_SIZE)
    pygame.draw.rect(screen, WHITE, ballrect)

    while not pygame.event.peek(eventtype=pygame.QUIT):
        pygame.event.pump()

        if pygame.event.peek(pygame.WINDOWRESIZED):
            width, height = pygame.display.get_surface().get_size()
            screen.fill(BLACK)
            pygame.event.clear()

        oldballrect = ballrect
        ballrect = next_rect(width, height)
        pygame.draw.rect(screen, BLACK, oldballrect)
        pygame.draw.rect(screen, WHITE, ballrect)
        pygame.display.update([oldballrect, ballrect])

if __name__ == "__main__":
    main()
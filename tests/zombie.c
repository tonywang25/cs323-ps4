// zombie.c
//
//   zombie COMMAND
//
// Fork a child and then run COMMAND.  The child exits immediately and thus
// becomes a zombie.

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main (int argc, char *argv[])
{
    int pid;

    if ((pid = fork()) < 0) {
	perror ("zombie");
	exit (EXIT_FAILURE);

    } else if (pid == 0) {
	exit (EXIT_SUCCESS);

    } else if (execvp (argv[1], &argv[1]) < 0) {
	perror ("zombie");
	exit (EXIT_FAILURE);
    }

    return EXIT_SUCCESS;
}

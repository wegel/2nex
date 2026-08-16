#include <stdio.h>

#include <readline/readline.h>

extern int _rl_completion_case_fold;

int
main(int argc, char **argv)
{
	const char *filename = NULL;
	int result;

	if (argc > 2) {
		fprintf(stderr, "usage: %s [INPUTRC]\n", argv[0]);
		return 2;
	}
	if (argc == 2)
		filename = argv[1];

	result = rl_read_init_file(filename);
	if (result != 0) {
		fprintf(stderr, "could not read inputrc: %d\n", result);
		return 1;
	}

	printf("%s\n", _rl_completion_case_fold ? "on" : "off");
	return 0;
}

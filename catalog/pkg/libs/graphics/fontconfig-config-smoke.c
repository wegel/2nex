#include <fontconfig/fontconfig.h>

#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static void
fail(const char *message)
{
	fprintf(stderr, "%s\n", message);
	exit(EXIT_FAILURE);
}

static void
make_path(char *path, size_t size, const char *root, const char *suffix)
{
	if (snprintf(path, size, "%s%s", root, suffix) >= (int)size)
		fail("smoke-test path is too long");
}

static FcConfig *
load_config(const char *root)
{
	FcConfig *config = FcConfigCreate();

	if (config == NULL)
		fail("could not create a Fontconfig configuration");
	FcConfigSetSysRoot(config, (const FcChar8 *)root);
	if (!FcConfigParseAndLoad(config, NULL, FcTrue)) {
		FcConfigDestroy(config);
		fail("could not load the Fontconfig configuration");
	}
	return config;
}

static int
list_has(FcStrList *list, const char *expected)
{
	FcChar8 *entry;
	int found = 0;

	if (list == NULL)
		fail("could not read a Fontconfig path list");
	while ((entry = FcStrListNext(list)) != NULL)
		if (strcmp((const char *)entry, expected) == 0)
			found = 1;
	FcStrListDone(list);
	return found;
}

static int
list_has_suffix(FcStrList *list, const char *expected)
{
	FcChar8 *entry;
	size_t expected_length = strlen(expected);
	int found = 0;

	if (list == NULL)
		fail("could not read a Fontconfig path list");
	while ((entry = FcStrListNext(list)) != NULL) {
		size_t entry_length = strlen((const char *)entry);

		if (entry_length >= expected_length &&
		    strcmp((const char *)entry + entry_length - expected_length,
			   expected) == 0)
			found = 1;
	}
	FcStrListDone(list);
	return found;
}

static void
expect_list_path(FcStrList *list, const char *expected)
{
	if (!list_has(list, expected)) {
		fprintf(stderr, "Fontconfig did not record %s\n", expected);
		exit(EXIT_FAILURE);
	}
}

static void
expect_list_suffix(FcStrList *list, const char *expected)
{
	if (!list_has_suffix(list, expected)) {
		fprintf(stderr, "Fontconfig did not record a path ending in %s\n",
			expected);
		exit(EXIT_FAILURE);
	}
}

static void
check_installed_graph(const char *root)
{
	char path[PATH_MAX];
	FcConfig *config = load_config(root);

	expect_list_suffix(FcConfigGetConfigFiles(config),
			   "/usr/share/fontconfig/fonts.conf");
	expect_list_suffix(FcConfigGetConfigFiles(config),
			   "/usr/share/fontconfig/conf.d/10-hinting-slight.conf");
	expect_list_suffix(FcConfigGetConfigFiles(config),
			   "/usr/share/fontconfig/conf.d/51-local.conf");

	make_path(path, sizeof(path), root, "/etc/fonts");
	expect_list_path(FcConfigGetConfigDirs(config), path);
	make_path(path, sizeof(path), root, "/run/fonts");
	expect_list_path(FcConfigGetConfigDirs(config), path);
	make_path(path, sizeof(path), root, "/usr/share/fontconfig");
	expect_list_path(FcConfigGetConfigDirs(config), path);

	FcConfigDestroy(config);
}

static void
expect_property(FcPattern *pattern, const char *property, const char *expected)
{
	FcChar8 *actual = NULL;
	FcResult result = FcPatternGetString(pattern, property, 0, &actual);

	if (strcmp(expected, "none") == 0) {
		if (result == FcResultNoMatch)
			return;
		fprintf(stderr, "%s was unexpectedly present\n", property);
		exit(EXIT_FAILURE);
	}
	if (result != FcResultMatch || strcmp((const char *)actual, expected) != 0) {
		fprintf(stderr, "%s was not %s\n", property, expected);
		exit(EXIT_FAILURE);
	}
}

static void
check_properties(const char *root, char **expected)
{
	static const char *const properties[] = {
		"uapi-main",
		"uapi-vendor",
		"uapi-runtime",
		"uapi-admin",
		"uapi-choice",
	};
	FcConfig *config = load_config(root);
	FcPattern *pattern = FcPatternCreate();
	size_t i;

	if (pattern == NULL)
		fail("could not create a Fontconfig pattern");
	if (!FcPatternAddString(pattern, FC_FAMILY, (const FcChar8 *)"probe") ||
	    !FcConfigSubstitute(config, pattern, FcMatchPattern))
		fail("could not apply the Fontconfig configuration");
	for (i = 0; i < sizeof(properties) / sizeof(properties[0]); i++)
		expect_property(pattern, properties[i], expected[i]);

	FcPatternDestroy(pattern);
	FcConfigDestroy(config);
}

static void
write_file(const char *path, const char *contents)
{
	FILE *file = fopen(path, "w");

	if (file == NULL) {
		perror(path);
		exit(EXIT_FAILURE);
	}
	if (fputs(contents, file) == EOF || fclose(file) != 0) {
		perror(path);
		exit(EXIT_FAILURE);
	}
}

static void
make_directory(const char *root, const char *suffix)
{
	char path[PATH_MAX];

	make_path(path, sizeof(path), root, suffix);
	if (mkdir(path, 0755) != 0 && errno != EEXIST) {
		perror(path);
		exit(EXIT_FAILURE);
	}
}

static void
write_rule(const char *root, const char *suffix,
	   const char *property, const char *value)
{
	char contents[1024];
	char path[PATH_MAX];

	if (snprintf(contents, sizeof(contents),
		     "<?xml version=\"1.0\"?>\n"
		     "<fontconfig><match target=\"pattern\">"
		     "<edit name=\"%s\" mode=\"assign_replace\">"
		     "<string>%s</string></edit>"
		     "</match></fontconfig>\n",
		     property, value) >= (int)sizeof(contents))
		fail("smoke-test rule is too long");
	make_path(path, sizeof(path), root, suffix);
	write_file(path, contents);
}

static void
write_main(const char *root, const char *suffix, const char *value,
	   int include_dropins)
{
	char contents[1024];
	char path[PATH_MAX];

	if (snprintf(contents, sizeof(contents),
		     "<?xml version=\"1.0\"?>\n"
		     "<fontconfig>"
		     "<match target=\"pattern\">"
		     "<edit name=\"uapi-main\" mode=\"assign_replace\">"
		     "<string>%s</string></edit>"
		     "</match>%s</fontconfig>\n",
		     value, include_dropins ? "<include>conf.d</include>" : "") >=
	    (int)sizeof(contents))
		fail("smoke-test main file is too long");
	make_path(path, sizeof(path), root, suffix);
	write_file(path, contents);
}

static void
remove_path(const char *root, const char *suffix, int directory)
{
	char path[PATH_MAX];
	int result;

	make_path(path, sizeof(path), root, suffix);
	result = directory ? rmdir(path) : unlink(path);
	if (result != 0) {
		perror(path);
		exit(EXIT_FAILURE);
	}
}

static void
check_new_higher_file(const char *root)
{
	static const char runtime_config[] =
		"<?xml version=\"1.0\"?>\n"
		"<fontconfig><match target=\"pattern\">"
		"<edit name=\"uapi-main\" mode=\"assign_replace\">"
		"<string>runtime-reload</string></edit>"
		"</match></fontconfig>\n";
	char directory[PATH_MAX];
	char file[PATH_MAX];
	char *expected[] = {
		"runtime-reload", "none", "none", "none", "none",
	};
	FcConfig *config = load_config(root);

	sleep(1);
	make_path(directory, sizeof(directory), root, "/run/fonts");
	if (mkdir(directory, 0755) != 0) {
		perror(directory);
		exit(EXIT_FAILURE);
	}
	make_path(file, sizeof(file), root, "/run/fonts/fonts.conf");
	write_file(file, runtime_config);
	if (FcConfigUptoDate(config))
		fail("Fontconfig did not notice a new higher-priority main file");
	FcConfigDestroy(config);

	check_properties(root, expected);
	if (unlink(file) != 0 || rmdir(directory) != 0)
		fail("could not clean the reload smoke-test files");
}

static void
run_layering_suite(const char *root)
{
	char empty_path[PATH_MAX];
	char *vendor[] = {
		"vendor", "vendor", "none", "none", "vendor",
	};
	char *runtime[] = {
		"runtime", "vendor", "runtime", "none", "runtime",
	};
	char *admin[] = {
		"admin", "vendor", "runtime", "admin", "admin",
	};
	char *dropin_mask[] = {
		"admin", "vendor", "runtime", "admin", "none",
	};
	char *main_mask[] = {
		"none", "none", "none", "none", "none",
	};
	char *runtime_with_admin[] = {
		"runtime", "vendor", "runtime", "admin", "none",
	};
	char *runtime_with_admin_choice[] = {
		"runtime", "vendor", "runtime", "admin", "runtime",
	};
	char *explicit_file[] = {
		"explicit", "none", "none", "none", "none",
	};
	char *explicit_path[] = {
		"path", "none", "none", "none", "none",
	};

	unsetenv("FONTCONFIG_FILE");
	unsetenv("FONTCONFIG_PATH");
	make_directory(root, "/etc");
	make_directory(root, "/run");

	write_main(root, "/usr/share/fontconfig/fonts.conf", "vendor", 1);
	write_rule(root, "/usr/share/fontconfig/conf.d/01-uapi-vendor.conf",
		   "uapi-vendor", "vendor");
	write_rule(root, "/usr/share/fontconfig/conf.d/40-uapi-choice.conf",
		   "uapi-choice", "vendor");
	check_properties(root, vendor);
	check_new_higher_file(root);

	make_directory(root, "/run/fonts");
	make_directory(root, "/run/fonts/conf.d");
	write_main(root, "/run/fonts/fonts.conf", "runtime", 1);
	write_rule(root, "/run/fonts/conf.d/02-uapi-runtime.conf",
		   "uapi-runtime", "runtime");
	write_rule(root, "/run/fonts/conf.d/40-uapi-choice.conf",
		   "uapi-choice", "runtime");
	check_properties(root, runtime);

	make_directory(root, "/etc/fonts");
	make_directory(root, "/etc/fonts/conf.d");
	write_main(root, "/etc/fonts/fonts.conf", "admin", 1);
	write_rule(root, "/etc/fonts/conf.d/03-uapi-admin.conf",
		   "uapi-admin", "admin");
	write_rule(root, "/etc/fonts/conf.d/40-uapi-choice.conf",
		   "uapi-choice", "admin");
	check_properties(root, admin);

	make_path(empty_path, sizeof(empty_path), root,
		  "/etc/fonts/conf.d/40-uapi-choice.conf");
	write_file(empty_path, "");
	check_properties(root, dropin_mask);

	make_path(empty_path, sizeof(empty_path), root, "/etc/fonts/fonts.conf");
	write_file(empty_path, "");
	check_properties(root, main_mask);
	remove_path(root, "/etc/fonts/fonts.conf", 0);
	check_properties(root, runtime_with_admin);
	remove_path(root, "/etc/fonts/conf.d/40-uapi-choice.conf", 0);
	check_properties(root, runtime_with_admin_choice);

	remove_path(root, "/etc/fonts/conf.d/03-uapi-admin.conf", 0);
	check_properties(root, runtime);
	remove_path(root, "/etc/fonts/conf.d", 1);
	remove_path(root, "/etc/fonts", 1);
	remove_path(root, "/run/fonts/fonts.conf", 0);
	remove_path(root, "/run/fonts/conf.d/02-uapi-runtime.conf", 0);
	remove_path(root, "/run/fonts/conf.d/40-uapi-choice.conf", 0);
	remove_path(root, "/run/fonts/conf.d", 1);
	remove_path(root, "/run/fonts", 1);
	check_properties(root, vendor);

	make_directory(root, "/override");
	write_main(root, "/override/fonts.conf", "explicit", 0);
	if (setenv("FONTCONFIG_FILE", "/override/fonts.conf", 1) != 0)
		fail("could not set FONTCONFIG_FILE");
	check_properties(root, explicit_file);
	unsetenv("FONTCONFIG_FILE");

	make_directory(root, "/override-path");
	write_main(root, "/override-path/fonts.conf", "path", 0);
	if (setenv("FONTCONFIG_PATH", "/override-path", 1) != 0)
		fail("could not set FONTCONFIG_PATH");
	check_properties(root, explicit_path);
	unsetenv("FONTCONFIG_PATH");

	remove_path(root, "/override/fonts.conf", 0);
	remove_path(root, "/override", 1);
	remove_path(root, "/override-path/fonts.conf", 0);
	remove_path(root, "/override-path", 1);
	remove_path(root, "/usr/share/fontconfig/conf.d/01-uapi-vendor.conf", 0);
	remove_path(root, "/usr/share/fontconfig/conf.d/40-uapi-choice.conf", 0);
	remove_path(root, "/etc", 1);
	remove_path(root, "/run", 1);
}

int
main(int argc, char **argv)
{
	if (argc == 3 && strcmp(argv[1], "graph") == 0) {
		check_installed_graph(argv[2]);
		return EXIT_SUCCESS;
	}
	if (argc == 3 && strcmp(argv[1], "reload") == 0) {
		check_new_higher_file(argv[2]);
		return EXIT_SUCCESS;
	}
	if (argc == 3 && strcmp(argv[1], "suite") == 0) {
		run_layering_suite(argv[2]);
		return EXIT_SUCCESS;
	}
	if (argc == 8 && strcmp(argv[1], "assert") == 0) {
		check_properties(argv[2], &argv[3]);
		return EXIT_SUCCESS;
	}

	fprintf(stderr,
		"usage: %s graph|reload|suite ROOT\n"
		"       %s assert ROOT MAIN VENDOR RUNTIME ADMIN CHOICE\n",
		argv[0], argv[0]);
	return EXIT_FAILURE;
}

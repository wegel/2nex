#include <gtk/gtk.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int
main(int argc, char **argv)
{
	GtkIMContext *context;
	GdkEventKey event = { 0 };
	PangoAttrList *attributes = NULL;
	char *preedit = NULL;
	int cursor = 0;
	gboolean handled;
	int status = EXIT_SUCCESS;

	if (argc != 2) {
		fprintf(stderr, "usage: %s EXPECTED_PREEDIT\n", argv[0]);
		return EXIT_FAILURE;
	}

	g_setenv("GTK_IM_MODULE", "multipress", TRUE);
	context = gtk_im_multicontext_new();
	event.type = GDK_KEY_PRESS;
	event.keyval = GDK_KEY_KP_2;
	handled = gtk_im_context_filter_keypress(context, &event);
	if (strcmp(argv[1], "masked") != 0 && !handled) {
		fprintf(stderr, "multipress module did not handle KP_2\n");
		status = EXIT_FAILURE;
	}

	gtk_im_context_get_preedit_string(context, &preedit, &attributes, &cursor);
	if (strcmp(argv[1], "masked") == 0) {
		if (preedit[0] != '\0') {
			fprintf(stderr, "expected empty preedit, got %s\n", preedit);
			status = EXIT_FAILURE;
		}
	} else if (strcmp(preedit, argv[1]) != 0) {
		fprintf(stderr, "expected preedit %s, got %s\n", argv[1], preedit);
		status = EXIT_FAILURE;
	}

	g_free(preedit);
	if (attributes != NULL)
		pango_attr_list_unref(attributes);
	g_object_unref(context);
	return status;
}

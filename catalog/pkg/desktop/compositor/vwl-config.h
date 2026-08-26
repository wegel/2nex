/* Taken from https://github.com/djpohly/dwl/issues/466 */
#define COLOR(hex)    { ((hex >> 24) & 0xFF) / 255.0f, \
                        ((hex >> 16) & 0xFF) / 255.0f, \
                        ((hex >> 8) & 0xFF) / 255.0f, \
                        (hex & 0xFF) / 255.0f }
/* appearance */
static const int sloppyfocus               = 1;  /* focus follows mouse */
static const int bypass_surface_visibility = 0;  /* 1 means idle inhibitors will disable idle tracking even if it's surface isn't visible  */
static const int fullscreen_idle_inhibit   = 1;  /* 1 inhibits idle whenever a visible client is fullscreen */
static const unsigned int borderpx         = 1;  /* border pixel of windows */
static const float rootcolor[]             = COLOR(0x1e1e2eff); /* mocha base */
static const float bordercolor[]           = COLOR(0x45475aff); /* mocha surface1 */
static const float focuscolor[]            = COLOR(0xb4befeff); /* mocha lavender */
static const float urgentcolor[]           = COLOR(0xf38ba8ff); /* mocha red */
/* This conforms to the xdg-protocol. Set the alpha to zero to restore the old behavior */
static const float fullscreen_bg[]         = {0.0f, 0.0f, 0.0f, 1.0f}; /* You can also use glsl colors */

/* tabbed header */
static const enum TabHdrPos tabhdr_position = TABHDR_BOTTOM;
static const float tabhdr_active_color[] = COLOR(0xb4befeff);       /* mocha lavender */
static const float tabhdr_inactive_color[] = COLOR(0x313244ff);     /* mocha surface0 */
static const float tabhdr_text_active_color[] = COLOR(0x1e1e2eff);  /* mocha base (dark on light) */
static const float tabhdr_text_inactive_color[] = COLOR(0xa6adc8ff); /* mocha subtext0 */
static const char tabhdr_font[] = "Sans 8";
static const int tabhdr_padding_top = 0;
static const int tabhdr_padding_bottom = 2;
static const int tabhdr_padding_left = 6;
static const int tabhdr_padding_right = 6;
static const TabTitleTransformRule tabhdr_title_transforms[] = {
	{ "zsh[0-9]+ ", "" },
	{ " - Firefox", " [firefox]" },
	{ NULL, NULL },
};

/* cursor */
static const int cursor_size = 48;

/* physical cursor continuity */
static const int enable_physical_cursor_gap_jumps = 1;

/* logging */
#ifdef __GNUC__
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wunused-variable"
#endif
static int log_level = WLR_INFO;
#ifdef __GNUC__
#pragma GCC diagnostic pop
#endif

/* NOTE: ALWAYS keep a rule declared even if you don't use rules (e.g leave at least one example) */
static const Rule rules[] = {
	/* app_id             title       workspace  monitor */
	{ "Gimp_EXAMPLE",     NULL,       0,         -1 },
};

/* layout(s) */
static const Layout layouts[] = {
	/* symbol     arrange function */
	{ "[]=",      tile },
	{ "[T]",      tabbed },
};

/* monitors */
/* NOTE: ALWAYS add a fallback rule, even if you are completely sure it won't be used */
static const MonitorRule monrules[] = {
	/* fallback - auto-configure any monitor */
	{ NULL,       1.0f, WL_OUTPUT_TRANSFORM_NORMAL,  -1,   -1, {
		.width_mm = 0,
		.height_mm = 0,
		.x_mm = 0,
		.y_mm = 0,
		.size_is_set = 0,
		.origin_is_set = 0,
	} },
};

/* virtual output rules - empty by default */
static const VirtualOutputRule vorules[] = {
	{ NULL,    NULL,     0,   0,   0,   0,   0.55f, 1,       &layouts[0],     (LENGTH(layouts) > 1) ? &layouts[1] : &layouts[0] },
};

/* keyboard */
static const struct xkb_rule_names xkb_rules = {
	.layout = "us",
	.variant = "altgr-intl",
	.options = NULL,
};

static const int repeat_rate = 70;
static const int repeat_delay = 220;

/* trackpad */
static const int tap_to_click = 1;
static const int tap_and_drag = 1;
static const int drag_lock = 1;
static const int natural_scrolling = 0;
static const int disable_while_typing = 1;
static const int left_handed = 0;
static const int middle_button_emulation = 0;
static const enum libinput_config_scroll_method scroll_method = LIBINPUT_CONFIG_SCROLL_2FG;
static const enum libinput_config_click_method click_method = LIBINPUT_CONFIG_CLICK_METHOD_BUTTON_AREAS;
static const uint32_t send_events_mode = LIBINPUT_CONFIG_SEND_EVENTS_ENABLED;
static const enum libinput_config_accel_profile accel_profile = LIBINPUT_CONFIG_ACCEL_PROFILE_ADAPTIVE;
static const double accel_speed = 0.0;
static const enum libinput_config_tap_button_map button_map = LIBINPUT_CONFIG_TAP_MAP_LRM;

/* modkey: left alt */
#define MODKEY WLR_MODIFIER_ALT

#define WORKSPACEKEY(KEY,SKEY,ID) \
	{ MODKEY,                    KEY,    view, {.ui = ID} }, \
	{ MODKEY|WLR_MODIFIER_SHIFT, SKEY,   tag,  {.ui = ID} }

/* helper for spawning shell commands in the pre dwm-5.0 fashion */
#define SHCMD(cmd) { .v = (const char*[]){ "/bin/sh", "-c", cmd, NULL } }

/* commands */
static const char *termcmd[] = { "foot", NULL };
static const char *menucmd[] = { "fuzzel", NULL };
static const char *screenshotcmd[] = {
	"/bin/sh",
	"-c",
	"dir=\"$HOME/Pictures/Screenshots\"; mkdir -p \"$dir\"; grim -g \"$(slurp)\" \"$dir/$(date +'%Y-%m-%d_%H-%M-%S').png\"",
	NULL,
};
static const char *screenshotclipboardcmd[] = {
	"/bin/sh",
	"-c",
	"slurp | grim -g - - | wl-copy",
	NULL,
};
static const char *lockcmd[] = {
	"/bin/sh",
	"-c",
	"sleep 1 && killall -USR1 swayidle",
	NULL,
};

static const Key keys[] = {
	/* Note that Shift changes certain key codes: c -> C, 2 -> at, etc. */
	/* modifier                  key                 function        argument */
	{ 0,                         XKB_KEY_Print,       spawn,         {.v = screenshotcmd} },
	{ MODKEY,                    XKB_KEY_Print,       spawn,         {.v = screenshotclipboardcmd} },
	{ MODKEY,                    XKB_KEY_d,          spawn,          {.v = menucmd} },
	{ MODKEY,                    XKB_KEY_Return,     spawn,          {.v = termcmd} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_L,          spawn,          {.v = lockcmd} },
	{ MODKEY,                    XKB_KEY_q,          killclient,     {0} },
	{ MODKEY,                    XKB_KEY_j,          focusstack,     {.i = -1} },
	{ MODKEY,                    XKB_KEY_k,          focusstack,     {.i = +1} },
	{ MODKEY,                    XKB_KEY_Left,       focusstack,     {.i = -1} },
	{ MODKEY,                    XKB_KEY_Right,      focusstack,     {.i = +1} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_j,          tabmove,        {.i = -1} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_k,          tabmove,        {.i = +1} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_Left,       tabmove,        {.i = -1} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_Right,      tabmove,        {.i = +1} },
	{ MODKEY,                    XKB_KEY_h,          setmfact,       {.f = -0.05f} },
	{ MODKEY,                    XKB_KEY_l,          setmfact,       {.f = +0.05f} },
	{ MODKEY,                    XKB_KEY_m,          zoom,           {0} },
	{ MODKEY,                    XKB_KEY_f,          togglefullscreen,{0} },
	{ MODKEY,                    XKB_KEY_t,          toggletabbed,   {.v = &layouts[1]} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_E,          quit,           {0} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_D,          debugstate,     {0} },
	{ MODKEY,                    XKB_KEY_comma,      focusmon,       {.i = WLR_DIRECTION_LEFT} },
	{ MODKEY,                    XKB_KEY_period,     focusmon,       {.i = WLR_DIRECTION_RIGHT} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_less,       tagmon,         {.i = WLR_DIRECTION_LEFT} },
	{ MODKEY|WLR_MODIFIER_SHIFT, XKB_KEY_greater,    tagmon,         {.i = WLR_DIRECTION_RIGHT} },
	{ MODKEY|WLR_MODIFIER_CTRL|WLR_MODIFIER_SHIFT, XKB_KEY_h,       moveworkspace, {.i = WLR_DIRECTION_LEFT} },
	{ MODKEY|WLR_MODIFIER_CTRL|WLR_MODIFIER_SHIFT, XKB_KEY_l,       moveworkspace, {.i = WLR_DIRECTION_RIGHT} },
	{ MODKEY|WLR_MODIFIER_CTRL|WLR_MODIFIER_SHIFT, XKB_KEY_j,       moveworkspace, {.i = WLR_DIRECTION_DOWN} },
	{ MODKEY|WLR_MODIFIER_CTRL|WLR_MODIFIER_SHIFT, XKB_KEY_k,       moveworkspace, {.i = WLR_DIRECTION_UP} },
	{ MODKEY|WLR_MODIFIER_CTRL|WLR_MODIFIER_SHIFT, XKB_KEY_Left,    moveworkspace, {.i = WLR_DIRECTION_LEFT} },
	{ MODKEY|WLR_MODIFIER_CTRL|WLR_MODIFIER_SHIFT, XKB_KEY_Right,   moveworkspace, {.i = WLR_DIRECTION_RIGHT} },
	{ MODKEY|WLR_MODIFIER_CTRL|WLR_MODIFIER_SHIFT, XKB_KEY_Down,    moveworkspace, {.i = WLR_DIRECTION_DOWN} },
	{ MODKEY|WLR_MODIFIER_CTRL|WLR_MODIFIER_SHIFT, XKB_KEY_Up,      moveworkspace, {.i = WLR_DIRECTION_UP} },
	WORKSPACEKEY(XKB_KEY_0, XKB_KEY_parenright, 0),
	WORKSPACEKEY(XKB_KEY_1, XKB_KEY_exclam,     1),
	WORKSPACEKEY(XKB_KEY_2, XKB_KEY_at,         2),
	WORKSPACEKEY(XKB_KEY_3, XKB_KEY_numbersign, 3),
	WORKSPACEKEY(XKB_KEY_4, XKB_KEY_dollar,     4),
	WORKSPACEKEY(XKB_KEY_5, XKB_KEY_percent,    5),
	WORKSPACEKEY(XKB_KEY_6, XKB_KEY_asciicircum,6),
	WORKSPACEKEY(XKB_KEY_7, XKB_KEY_ampersand,  7),
	WORKSPACEKEY(XKB_KEY_8, XKB_KEY_asterisk,   8),
	WORKSPACEKEY(XKB_KEY_9, XKB_KEY_parenleft,  9),
	/* Ctrl-Alt-Backspace and Ctrl-Alt-Fx used to be handled by X server */
	{ WLR_MODIFIER_CTRL|WLR_MODIFIER_ALT,XKB_KEY_Terminate_Server, quit, {0} },
	/* Ctrl-Alt-Fx is used to switch to another VT, if you don't know what a VT is
	 * do not remove them.
	 */
#define CHVT(n) { WLR_MODIFIER_CTRL|WLR_MODIFIER_ALT,XKB_KEY_XF86Switch_VT_##n, chvt, {.ui = (n)} }
	CHVT(1), CHVT(2), CHVT(3), CHVT(4), CHVT(5), CHVT(6),
	CHVT(7), CHVT(8), CHVT(9), CHVT(10), CHVT(11), CHVT(12),
};


static const Button buttons[] = {
	{ 0, 0, NULL, {0} },
};

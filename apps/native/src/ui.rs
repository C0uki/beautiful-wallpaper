//! The native surfaces' markup: the palette, the bar, the readout, the banners
//! and the dock, in Slint. One `slint!` for all of them, so `Palette` and
//! `Icons` are the same global to every window.

slint::slint! {
    // ---- palette ---------------------------------------------------------------

    // The colours the native surfaces share.

    export global Palette {
        // Set from Rust, as the shell's theme goes light or dark.
        in property <bool> light;
        // The webview bar's `hug` style, which is Apple's fixed greys rather than
        // anything taken from the wallpaper.
        out property <color> background: light ? #ececec : #282828;
        out property <color> foreground: light ? #1d1d1f : #e0e0e0;
        out property <color> quiet: light ? #1d1d1fa0 : #e0e0e0a0;
        out property <color> hover: light ? #0000000f : #ffffff1a;
        out property <color> accent: light ? #007aff : #0a84ff;
        out property <color> filled: light ? #00000014 : #ffffff26;
        out property <color> outline: light ? #00000026 : #ffffff33;
        // The readout's block, its track and its level.
        out property <color> popover: light ? #ffffff : #2a2a2c;
        out property <color> track: light ? #d1d1d6 : #48484a;
        out property <color> level: light ? #1d1d1f : white;
        out property <color> muted: #8e8e93;
        // A critical notification's tile.
        out property <color> danger: light ? #ff3b30 : #ff453a;
    }

    // ---- bar -------------------------------------------------------------------

    // The bar, drawn natively: the same strip the webview bar draws, in the order
    // the default config puts it in — along the top, or down the left side.



    // Set from Rust, out of the same icon set the webview surfaces draw.
    export global Icons {
        in property <image> left-panel;
        in property <image> right-panel;
        in property <image> desktop;
        in property <image> wallpaper;
        in property <image> speed;
        in property <image> memory;
        in property <image> storage;
        in property <image> down;
        in property <image> up;
        in property <image> battery;
        in property <image> battery-charging;
        in property <image> photos;
        in property <image> shuffle;
        in property <image> settings;
        in property <image> play;
        in property <image> pause;
        in property <image> next;
    }

    export struct TrayItem {
        icon: image,
    }

    export struct WorkspaceItem {
        name: string,
        focused: bool,
        populated: bool,
    }

    component Item inherits Rectangle {
        in property <image> icon;
        in property <string> text;
        in property <length> icon-size: 16px;
        in property <color> tint: Palette.foreground;
        in property <length> max-text: 400px;
        callback clicked;

        height: 24px;
        border-radius: 6px;
        background: area.has-hover ? Palette.hover : transparent;

        HorizontalLayout {
            padding-left: 6px;
            padding-right: 6px;
            spacing: 4px;
            if root.icon.width > 0: Image {
                source: root.icon;
                colorize: root.tint;
                width: root.icon-size;
                height: root.icon-size;
                y: (parent.height - self.height) / 2;
            }
            if root.text != "": Text {
                text: root.text;
                color: root.tint;
                vertical-alignment: center;
                overflow: elide;
                max-width: root.max-text;
            }
        }

        area := TouchArea {
            clicked => {
                root.clicked();
            }
        }
    }

    // One of the vertical bar's buttons: the icon, with a short reading under it.
    component Cell inherits Rectangle {
        in property <image> icon;
        in property <string> text;
        in property <length> icon-size: 16px;
        callback clicked;

        height: root.text == "" ? 26px : 36px;
        border-radius: 6px;
        background: area.has-hover ? Palette.hover : transparent;

        VerticalLayout {
            alignment: center;
            spacing: 1px;
            if root.icon.width > 0: HorizontalLayout {
                alignment: center;
                Image {
                    source: root.icon;
                    colorize: Palette.foreground;
                    width: root.icon-size;
                    height: root.icon-size;
                }
            }
            if root.text != "": Text {
                text: root.text;
                color: Palette.foreground;
                font-size: 10px;
                horizontal-alignment: center;
            }
        }

        area := TouchArea {
            clicked => {
                root.clicked();
            }
        }
    }

    export component Bar inherits Window {
        in property <string> active-title;
        in property <string> cpu;
        in property <string> memory;
        in property <string> disk;
        in property <string> down;
        in property <string> up;
        in property <string> battery;
        in property <bool> charging;
        in property <string> date;
        in property <string> time;
        // Down the left edge, where only icons and the clock, a line at a time,
        // fit.
        in property <bool> vertical;
        in property <string> vertical-clock;
        // What the centre says over the desktop, in the shell's language.
        in property <string> desktop-label;
        in property <image> weather-icon;
        in property <string> weather;
        in property <string> media-title;
        in property <bool> media-playing;
        in property <[TrayItem]> tray;
        in property <[WorkspaceItem]> workspaces;
        callback clicked(string);
        callback tray-clicked(int, bool);

        title: "bw-bar-native";
        no-frame: true;
        always-on-top: true;
        background: Palette.background;
        default-font-family: "Segoe UI Variable Text";
        default-font-size: 13px;

        if !root.vertical: Rectangle {
            HorizontalLayout {
                x: 6px;
                width: self.preferred-width;
                spacing: 2px;
                Item {
                    icon: Icons.left-panel;
                    clicked => {
                        root.clicked("sidebarLeft");
                    }
                }
                if root.media-title != "": HorizontalLayout {
                    Item {
                        icon: root.media-playing ? Icons.pause : Icons.play;
                        clicked => {
                            root.clicked("playPause");
                        }
                    }
                    Item {
                        text: root.media-title;
                        max-text: 220px;
                    }
                    Item {
                        icon: Icons.next;
                        clicked => {
                            root.clicked("next");
                        }
                    }
                }
                if root.weather != "": Item {
                    icon: root.weather-icon;
                    icon-size: 17px;
                    text: root.weather;
                }
            }

            // Centred on the screen, not on what the two sides leave over, so the
            // clock does not drift as the title grows.
            HorizontalLayout {
                width: min(self.preferred-width, root.width * 60%);
                x: (root.width - self.width) / 2;
                spacing: 4px;
                for workspace in root.workspaces: Rectangle {
                    y: (parent.height - self.height) / 2;
                    height: 22px;
                    min-width: workspace.focused ? 30px : 22px;
                    border-radius: 11px;
                    background: workspace.focused ? Palette.accent : workspace.populated ? Palette.filled : transparent;
                    border-width: workspace.focused || workspace.populated ? 0px : 1px;
                    border-color: Palette.outline;
                    HorizontalLayout {
                        padding-left: 6px;
                        padding-right: 6px;
                        Text {
                            text: workspace.name;
                            color: workspace.focused ? white : Palette.quiet;
                            font-size: 11px;
                            font-weight: 700;
                            horizontal-alignment: center;
                            vertical-alignment: center;
                        }
                    }
                }
                Item {
                    icon: root.active-title != "" ? Icons.desktop : Icons.wallpaper;
                    text: root.active-title != "" ? root.active-title : root.desktop-label;
                    tint: root.active-title != "" ? Palette.foreground : Palette.quiet;
                }
            }

            HorizontalLayout {
                width: self.preferred-width;
                x: root.width - self.width - 6px;
                spacing: 2px;
                for icon[index] in root.tray: Rectangle {
                    width: 26px;
                    height: 24px;
                    y: (parent.height - self.height) / 2;
                    border-radius: 6px;
                    background: tray-area.has-hover ? Palette.hover : transparent;
                    if icon.icon.width > 0: Image {
                        source: icon.icon;
                        x: (parent.width - self.width) / 2;
                        y: (parent.height - self.height) / 2;
                        width: 16px;
                        height: 16px;
                    }
                    if icon.icon.width == 0: Rectangle {
                        x: (parent.width - self.width) / 2;
                        y: (parent.height - self.height) / 2;
                        width: 6px;
                        height: 6px;
                        border-radius: 3px;
                        background: Palette.quiet;
                    }
                    tray-area := TouchArea {
                        clicked => {
                            root.tray-clicked(index, false);
                        }
                        pointer-event(event) => {
                            if event.button == PointerEventButton.right && event.kind == PointerEventKind.up {
                                root.tray-clicked(index, true);
                            }
                        }
                    }
                }
                Item {
                    icon: Icons.speed;
                    text: root.cpu;
                }
                Item {
                    icon: Icons.memory;
                    text: root.memory;
                }
                Item {
                    icon: Icons.storage;
                    text: root.disk;
                }
                Item {
                    icon: Icons.down;
                    icon-size: 14px;
                    text: root.down;
                }
                Item {
                    icon: Icons.up;
                    icon-size: 14px;
                    text: root.up;
                }
                if root.battery != "": Item {
                    icon: root.charging ? Icons.battery-charging : Icons.battery;
                    text: root.battery;
                }
                Item {
                    icon: Icons.photos;
                    clicked => {
                        root.clicked("wallpaperSelector");
                    }
                }
                Item {
                    icon: Icons.shuffle;
                    clicked => {
                        root.clicked("randomWallpaper");
                    }
                }
                Item {
                    icon: Icons.settings;
                    clicked => {
                        root.clicked("settings");
                    }
                }
                Item {
                    icon: Icons.right-panel;
                    clicked => {
                        root.clicked("sidebarRight");
                    }
                }
                Item {
                    text: root.date + "  " + root.time;
                }
            }
        }

        if root.vertical: Rectangle {
            VerticalLayout {
                y: 6px;
                height: self.preferred-height;
                padding-left: 2px;
                padding-right: 2px;
                spacing: 2px;
                Cell {
                    icon: Icons.left-panel;
                    clicked => {
                        root.clicked("sidebarLeft");
                    }
                }
                if root.media-title != "": Cell {
                    icon: root.media-playing ? Icons.pause : Icons.play;
                    clicked => {
                        root.clicked("playPause");
                    }
                }
                if root.weather != "": Cell {
                    icon: root.weather-icon;
                    icon-size: 17px;
                    text: root.weather;
                }
                for workspace in root.workspaces: HorizontalLayout {
                    alignment: center;
                    Rectangle {
                        width: 22px;
                        height: workspace.focused ? 30px : 22px;
                        border-radius: 11px;
                        background: workspace.focused ? Palette.accent : workspace.populated ? Palette.filled : transparent;
                        border-width: workspace.focused || workspace.populated ? 0px : 1px;
                        border-color: Palette.outline;
                        Text {
                            text: workspace.name;
                            color: workspace.focused ? white : Palette.quiet;
                            font-size: 11px;
                            font-weight: 700;
                            horizontal-alignment: center;
                            vertical-alignment: center;
                        }
                    }
                }
            }

            VerticalLayout {
                y: root.height - self.height - 6px;
                height: self.preferred-height;
                padding-left: 2px;
                padding-right: 2px;
                spacing: 2px;
                for icon[index] in root.tray: HorizontalLayout {
                    alignment: center;
                    Rectangle {
                        width: 26px;
                        height: 24px;
                        border-radius: 6px;
                        background: vertical-tray-area.has-hover ? Palette.hover : transparent;
                        if icon.icon.width > 0: Image {
                            source: icon.icon;
                            x: (parent.width - self.width) / 2;
                            y: (parent.height - self.height) / 2;
                            width: 16px;
                            height: 16px;
                        }
                        if icon.icon.width == 0: Rectangle {
                            x: (parent.width - self.width) / 2;
                            y: (parent.height - self.height) / 2;
                            width: 6px;
                            height: 6px;
                            border-radius: 3px;
                            background: Palette.quiet;
                        }
                        vertical-tray-area := TouchArea {
                            clicked => {
                                root.tray-clicked(index, false);
                            }
                            pointer-event(event) => {
                                if event.button == PointerEventButton.right && event.kind == PointerEventKind.up {
                                    root.tray-clicked(index, true);
                                }
                            }
                        }
                    }
                }
                Cell {
                    icon: Icons.speed;
                    text: root.cpu;
                }
                Cell {
                    icon: Icons.memory;
                    text: root.memory;
                }
                if root.battery != "": Cell {
                    icon: root.charging ? Icons.battery-charging : Icons.battery;
                    text: root.battery;
                }
                Cell {
                    icon: Icons.photos;
                    clicked => {
                        root.clicked("wallpaperSelector");
                    }
                }
                Cell {
                    icon: Icons.shuffle;
                    clicked => {
                        root.clicked("randomWallpaper");
                    }
                }
                Cell {
                    icon: Icons.settings;
                    clicked => {
                        root.clicked("settings");
                    }
                }
                Cell {
                    icon: Icons.right-panel;
                    clicked => {
                        root.clicked("sidebarRight");
                    }
                }
                Text {
                    text: root.vertical-clock;
                    color: Palette.foreground;
                    font-size: 12px;
                    font-weight: 600;
                    horizontal-alignment: center;
                }
            }
        }
    }

    // ---- osd -------------------------------------------------------------------

    // The volume and brightness readout, drawn as the webview's is: the glyph,
    // Control Center's tall track filled in white, and the level. The window is
    // exactly the block; Windows rounds its corners and gives it its shadow.


    export component Osd inherits Window {
        in property <image> glyph;
        /// 0 to 1.
        in property <float> level;
        in property <string> value;
        /// Still shows where the level would be, greyed, so unmuting is no surprise.
        in property <bool> muted;

        title: "bw-osd-native";
        no-frame: true;
        always-on-top: true;
        background: Palette.popover;
        width: 320px;
        height: 46px;
        default-font-family: "Segoe UI Variable Text";
        default-font-size: 13px;

        HorizontalLayout {
            padding-left: 14px;
            padding-right: 14px;
            padding-top: 12px;
            padding-bottom: 12px;
            spacing: 12px;
            Image {
                source: root.glyph;
                colorize: root.muted ? Palette.muted : Palette.foreground;
                width: 20px;
                height: 20px;
                y: (parent.height - self.height) / 2;
            }
            Rectangle {
                horizontal-stretch: 1;
                height: 22px;
                border-radius: 11px;
                background: Palette.track;
                clip: true;
                Rectangle {
                    x: 0;
                    width: parent.width * root.level;
                    height: parent.height;
                    border-radius: 11px;
                    background: root.muted ? Palette.muted : Palette.level;
                    animate width {
                        duration: 150ms;
                        easing: ease-out;
                    }
                }
            }
            Text {
                text: root.value;
                color: Palette.quiet;
                vertical-alignment: center;
                horizontal-alignment: right;
                min-width: 24px;
            }
        }
    }

    // ---- toast -----------------------------------------------------------------

    // One notification banner, drawn as the webview's is, a Mac's: the app's tile
    // on the left — red for a critical one — the title in bold, the app and how
    // long ago, the body, and the close button only under the pointer.
    //
    // One window each. The renderer cannot make a window see-through, so the
    // stack is windows laid one under another, each exactly its banner; Windows
    // rounds their corners and gives them their shadows.


    export component Toast inherits Window {
        in property <image> glyph;
        in property <bool> critical;
        in property <string> summary;
        in property <string> app;
        in property <string> age;
        in property <string> body;
        /// "+2 more", or empty.
        in property <string> more;
        callback close();
        /// The pointer went down, moved while down, or came up: the window is
        /// dragged by where the pointer is on the screen, which Rust reads, since
        /// a position inside the window moves with it.
        callback drag-start();
        callback drag-move();
        callback drag-end();

        title: "bw-toast-native";
        no-frame: true;
        always-on-top: true;
        background: Palette.background;
        width: 344px;
        height: content.preferred-height;
        default-font-family: "Segoe UI Variable Text";
        default-font-size: 13px;

        swipe := TouchArea {
            pointer-event(event) => {
                if event.button == PointerEventButton.left && event.kind == PointerEventKind.down {
                    root.drag-start();
                }
                if event.button == PointerEventButton.left && event.kind == PointerEventKind.up {
                    root.drag-end();
                }
            }
            moved => {
                if self.pressed {
                    root.drag-move();
                }
            }
        }

        content := HorizontalLayout {
            padding: 12px;
            spacing: 10px;
            alignment: start;
            Rectangle {
                width: 34px;
                height: 34px;
                border-radius: 8px;
                background: root.critical ? Palette.danger : Palette.track;
                Image {
                    source: root.glyph;
                    colorize: Palette.foreground;
                    width: 18px;
                    height: 18px;
                }
            }
            VerticalLayout {
                spacing: 2px;
                horizontal-stretch: 1;
                HorizontalLayout {
                    spacing: 8px;
                    Text {
                        text: root.app;
                        color: Palette.quiet;
                        font-size: 10px;
                        overflow: elide;
                        horizontal-stretch: 1;
                    }
                    Text {
                        text: root.age;
                        color: Palette.quiet;
                        font-size: 10px;
                    }
                }
                Text {
                    text: root.summary;
                    color: Palette.foreground;
                    font-weight: 600;
                    wrap: word-wrap;
                }
                // Two lines, then elide: a banner is a glance, not a document.
                if root.body != "": Text {
                    text: root.body;
                    color: Palette.quiet;
                    font-size: 11.5px;
                    wrap: word-wrap;
                    overflow: elide;
                    height: min(self.preferred-height, 32px);
                }
                if root.more != "": Text {
                    text: root.more;
                    color: Palette.quiet;
                    font-size: 10px;
                }
            }
        }

        // Over the swipe area, so a press on it closes rather than drags. Shown
        // only while the pointer is on the banner, as a Mac's is.
        Rectangle {
            x: parent.width - self.width - 6px;
            y: 6px;
            width: 20px;
            height: 20px;
            border-radius: 10px;
            opacity: swipe.has-hover || close-area.has-hover ? 1 : 0;
            background: close-area.has-hover ? Palette.hover : Palette.track;
            Text {
                text: "×";
                color: Palette.foreground;
                font-size: 13px;
            }
            close-area := TouchArea {
                clicked => {
                    root.close();
                }
            }
        }
    }

    // ---- dock ------------------------------------------------------------------

    // The dock, drawn as the webview's is: the pinned applications, a separator,
    // the running ones, then the playing track and the button that keeps the dock
    // up. The window is exactly the dock; Windows rounds its corners and gives it
    // its shadow.


    export struct DockItem {
        icon: image,
        name: string,
        running: bool,
        active: bool,
    }

    component DockIcon inherits Rectangle {
        in property <DockItem> item;
        in property <length> size;
        in property <image> fallback;
        in property <bool> drop-target;
        in property <bool> dragging;
        /// Pinned icons can be dragged along the others.
        in property <bool> movable;
        /// How many places the pointer has been dragged across, while dragging.
        callback pressed();
        callback dragged(length);
        callback released(bool);
        callback secondary();

        width: root.size;
        height: root.size;
        border-radius: 10px;
        border-width: root.drop-target ? 2px : 0px;
        border-color: Palette.accent;
        background: area.pressed ? Palette.hover : transparent;
        opacity: root.dragging ? 0.4 : 1;

        Image {
            source: root.item.icon.width > 0 ? root.item.icon : root.fallback;
            width: root.size * 0.8;
            height: root.size * 0.8;
            x: (parent.width - self.width) / 2;
            y: (parent.height - self.height) / 2 - root.size * 0.03;
        }
        if root.item.running: Rectangle {
            width: 4px;
            height: 4px;
            border-radius: 2px;
            x: (parent.width - self.width) / 2;
            y: parent.height - self.height;
            background: Palette.foreground;
        }

        area := TouchArea {
            pointer-event(event) => {
                if event.button == PointerEventButton.left && event.kind == PointerEventKind.down {
                    root.pressed();
                }
                if event.button == PointerEventButton.left && event.kind == PointerEventKind.up {
                    root.released(self.has-hover);
                }
                if event.button == PointerEventButton.right && event.kind == PointerEventKind.up {
                    root.secondary();
                }
            }
            moved => {
                if self.pressed && root.movable {
                    root.dragged(self.mouse-x - self.pressed-x);
                }
            }
        }
    }

    component Separator inherits Rectangle {
        width: 1px;
        background: Palette.outline;
    }

    export component Dock inherits Window {
        in property <[DockItem]> pinned-apps;
        in property <[DockItem]> running-apps;
        in property <length> icon-size: 44px;
        in property <image> fallback;
        in property <bool> show-media;
        in property <string> media-title;
        in property <image> media-glyph;
        in property <bool> show-keep;
        in property <bool> kept;
        in property <image> keep-glyph;
        /// The pinned icon being dragged, and the place it would take: -1 when
        /// nothing is.
        in-out property <int> dragged: -1;
        in-out property <int> over: -1;
        callback activate(bool, int);
        callback pin(bool, int);
        callback move(int, int);
        callback media();
        callback keep();
        callback hovered(bool);
        /// The size the dock wants, which a hidden window does not take on until
        /// it is shown again.
        out property <length> wanted-width: row.preferred-width;
        out property <length> wanted-height: row.preferred-height;

        title: "bw-dock-native";
        no-frame: true;
        always-on-top: true;
        background: Palette.popover;
        width: row.preferred-width;
        height: row.preferred-height;
        default-font-family: "Segoe UI Variable Text";
        default-font-size: 13px;

        property <length> slot: root.icon-size + 2px;

        hover := TouchArea {
            changed has-hover => {
                root.hovered(self.has-hover);
            }

            row := HorizontalLayout {
                padding-left: 6px;
                padding-right: 6px;
                padding-top: 4px;
                padding-bottom: 4px;
                spacing: 2px;

                for item[index] in root.pinned-apps: DockIcon {
                    item: item;
                    size: root.icon-size;
                    fallback: root.fallback;
                    movable: true;
                    dragging: root.dragged == index;
                    drop-target: root.dragged >= 0 && root.over == index && root.dragged != index;
                    pressed => {
                        root.dragged = -1;
                        root.over = -1;
                    }
                    dragged(distance) => {
                        if abs(distance) > 6px || root.dragged >= 0 {
                            root.dragged = index;
                            root.over = max(0, min(root.pinned-apps.length - 1, index + round(distance / root.slot)));
                        }
                    }
                    released(inside) => {
                        if root.dragged >= 0 {
                            root.move(root.dragged, root.over);
                            root.dragged = -1;
                            root.over = -1;
                        } else if inside {
                            root.activate(true, index);
                        }
                    }
                    secondary => {
                        root.pin(true, index);
                    }
                }

                if root.pinned-apps.length > 0 && root.running-apps.length > 0: Separator {}

                for item[index] in root.running-apps: DockIcon {
                    item: item;
                    size: root.icon-size;
                    fallback: root.fallback;
                    released(inside) => {
                        if inside {
                            root.activate(false, index);
                        }
                    }
                    secondary => {
                        root.pin(false, index);
                    }
                }

                if root.show-media && root.media-title != "": Separator {}
                if root.show-media && root.media-title != "": Rectangle {
                    border-radius: 8px;
                    background: media-area.has-hover ? Palette.hover : transparent;
                    HorizontalLayout {
                        padding-left: 10px;
                        padding-right: 10px;
                        spacing: 6px;
                        Image {
                            source: root.media-glyph;
                            colorize: Palette.foreground;
                            width: 20px;
                            height: 20px;
                            y: (parent.height - self.height) / 2;
                        }
                        Text {
                            text: root.media-title;
                            color: Palette.quiet;
                            font-size: 11px;
                            vertical-alignment: center;
                            overflow: elide;
                            max-width: 180px;
                        }
                    }
                    media-area := TouchArea {
                        clicked => {
                            root.media();
                        }
                    }
                }

                if root.show-keep: Separator {}
                if root.show-keep: Rectangle {
                    width: 38px;
                    border-radius: 8px;
                    background: keep-area.has-hover ? Palette.hover : transparent;
                    Image {
                        source: root.keep-glyph;
                        colorize: root.kept ? Palette.accent : Palette.quiet;
                        width: 18px;
                        height: 18px;
                    }
                    keep-area := TouchArea {
                        clicked => {
                            root.keep();
                        }
                    }
                }
            }
        }
    }
}

// The sidebar's header.
//
// The original draws the current wallpaper behind an avatar, the user's name
// and the uptime, with a row of shell buttons in the corner. Clicking the image
// picks a different banner; right-clicking clears it back to the wallpaper.

import { IconButton, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { backend } from "../../shell/backend";
import { actions, useShell } from "../../shell/store";

export function Banner() {
  const config = useShell((state) => state.config.sidebar);
  const wallpaper = useShell((state) => state.wallpaper);
  const info = useShell((state) => state.systemInfo);
  const states = useShell((state) => state.states);

  const image = config.bannerImage || wallpaper.path;

  const buttons = (
    <div className="bw-banner-buttons">
      <IconButton
        icon="wallpaper"
        size={34}
        label={tr("Wallpapers")}
        onClick={() => void actions.toggleState("wallpaperSelectorOpen")}
      />
      <IconButton
        icon="settings"
        size={34}
        label={tr("Settings")}
        onClick={() => {
          // The sidebar gets out of the way: the settings window is where the
          // user is going, and two panels fighting for focus helps nobody.
          void actions.setState("sidebarRightOpen", false);
          void actions.toggleState("settingsOpen");
        }}
      />
      <IconButton
        icon="power_settings_new"
        size={34}
        label={tr("Session")}
        onClick={() => void actions.setState("sessionOpen", true)}
      />
    </div>
  );

  if (!config.banner) {
    return (
      <div className="bw-banner bw-banner-plain">
        <div className="bw-banner-text">
          <span className="bw-banner-name">
            {info ? `${info.username}@${info.hostname}` : " "}
          </span>
          <span className="bw-banner-uptime">
            {info ? tr("Up • %1").replace("%1", info.uptime) : " "}
          </span>
        </div>
        {buttons}
      </div>
    );
  }

  return (
    <div className="bw-banner" data-editing={states.widgetEditMode}>
      <div
        className="bw-banner-image"
        style={
          image
            ? // Quoted: an asset URL can be a data: URL, and the ';' and ','
              // inside one terminate an unquoted css url().
              { backgroundImage: `url("${backend().assetUrl(image)}")` }
            : undefined
        }
        role="img"
        aria-label={tr("Banner")}
        onContextMenu={(event) => {
          event.preventDefault();
          // Right-click clears the override, as upstream does — back to
          // whatever the wallpaper currently is.
          void actions.setConfigValue("sidebar.bannerImage", "");
        }}
      />

      <div className="bw-banner-foot">
        <div className="bw-banner-avatar">
          {config.profile.avatarPath ? (
            <img
              src={backend().assetUrl(config.profile.avatarPath)}
              alt=""
              onError={(event) => {
                // A configured path that no longer resolves falls back to the
                // glyph rather than showing a broken image.
                event.currentTarget.style.display = "none";
              }}
            />
          ) : (
            <Symbol name="account_circle" size={34} />
          )}
        </div>

        <div className="bw-banner-text">
          <span className="bw-banner-name">
            {info ? `${info.username}@${info.hostname}` : " "}
          </span>
          <span className="bw-banner-uptime">
            {info ? tr("Up • %1").replace("%1", info.uptime) : " "}
          </span>
        </div>

        {buttons}
      </div>
    </div>
  );
}

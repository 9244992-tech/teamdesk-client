Name:       teamdesk
Version:    1.4.8
Release:    0
Summary:    RPM package
License:    GPL-3.0
URL:        https://teamdesk.com
Vendor:     teamdesk <info@teamdesk.com>
Requires:   gtk3 libxcb1 libXfixes3 alsa-utils libXtst6 libva2 pam gstreamer-plugins-base gstreamer-plugin-pipewire
Recommends: libayatana-appindicator3-1 xdotool
Provides:   libdesktop_drop_plugin.so()(64bit), libdesktop_multi_window_plugin.so()(64bit), libfile_selector_linux_plugin.so()(64bit), libflutter_custom_cursor_plugin.so()(64bit), libflutter_linux_gtk.so()(64bit), libscreen_retriever_plugin.so()(64bit), libtray_manager_plugin.so()(64bit), liburl_launcher_linux_plugin.so()(64bit), libwindow_manager_plugin.so()(64bit), libwindow_size_plugin.so()(64bit), libtexture_rgba_renderer_plugin.so()(64bit)

# https://docs.fedoraproject.org/en-US/packaging-guidelines/Scriptlets/

%description
The best open-source remote desktop client software, written in Rust.

%prep
# we have no source, so nothing here

%build
# we have no source, so nothing here

# %global __python %{__python3}

%install

mkdir -p "%{buildroot}/usr/share/teamdesk" && cp -r ${HBB}/flutter/build/linux/x64/release/bundle/* -t "%{buildroot}/usr/share/teamdesk"
mkdir -p "%{buildroot}/usr/bin"
install -Dm 644 $HBB/res/teamdesk.service -t "%{buildroot}/usr/share/teamdesk/files"
install -Dm 644 $HBB/res/teamdesk.desktop -t "%{buildroot}/usr/share/teamdesk/files"
install -Dm 644 $HBB/res/teamdesk-link.desktop -t "%{buildroot}/usr/share/teamdesk/files"
install -Dm 644 $HBB/res/128x128@2x.png "%{buildroot}/usr/share/icons/hicolor/256x256/apps/teamdesk.png"
install -Dm 644 $HBB/res/scalable.svg "%{buildroot}/usr/share/icons/hicolor/scalable/apps/teamdesk.svg"

%files
/usr/share/teamdesk/*
/usr/share/teamdesk/files/teamdesk.service
/usr/share/icons/hicolor/256x256/apps/teamdesk.png
/usr/share/icons/hicolor/scalable/apps/teamdesk.svg
/usr/share/teamdesk/files/teamdesk.desktop
/usr/share/teamdesk/files/teamdesk-link.desktop

%changelog
# let's skip this for now

%pre
# can do something for centos7
case "$1" in
  1)
    # for install
  ;;
  2)
    # for upgrade
    systemctl stop teamdesk || true
  ;;
esac

%post
cp /usr/share/teamdesk/files/teamdesk.service /etc/systemd/system/teamdesk.service
cp /usr/share/teamdesk/files/teamdesk.desktop /usr/share/applications/
cp /usr/share/teamdesk/files/teamdesk-link.desktop /usr/share/applications/
ln -sf /usr/share/teamdesk/teamdesk /usr/bin/teamdesk
systemctl daemon-reload
systemctl enable teamdesk
systemctl start teamdesk
update-desktop-database

%preun
case "$1" in
  0)
    # for uninstall
    systemctl stop teamdesk || true
    systemctl disable teamdesk || true
    rm /etc/systemd/system/teamdesk.service || true
  ;;
  1)
    # for upgrade
  ;;
esac

%postun
case "$1" in
  0)
    # for uninstall
    rm /usr/bin/teamdesk || true
    rmdir /usr/lib/teamdesk || true
    rmdir /usr/local/teamdesk || true
    rmdir /usr/share/teamdesk || true
    rm /usr/share/applications/teamdesk.desktop || true
    rm /usr/share/applications/teamdesk-link.desktop || true
    update-desktop-database
  ;;
  1)
    # for upgrade
    rmdir /usr/lib/teamdesk || true
    rmdir /usr/local/teamdesk || true
  ;;
esac

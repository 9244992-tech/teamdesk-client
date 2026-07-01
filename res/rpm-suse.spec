Name:       teamdesk
Version:    1.1.9
Release:    0
Summary:    RPM package
License:    GPL-3.0
Requires:   gtk3 libxcb1 libXfixes3 alsa-utils libXtst6 libva2 pam gstreamer-plugins-base gstreamer-plugin-pipewire
Recommends: libayatana-appindicator3-1 xdotool

# https://docs.fedoraproject.org/en-US/packaging-guidelines/Scriptlets/

%description
The best open-source remote desktop client software, written in Rust.

%prep
# we have no source, so nothing here

%build
# we have no source, so nothing here

%global __python %{__python3}

%install
mkdir -p %{buildroot}/usr/bin/
mkdir -p %{buildroot}/usr/share/teamdesk/
mkdir -p %{buildroot}/usr/share/teamdesk/files/
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps/
mkdir -p %{buildroot}/usr/share/icons/hicolor/scalable/apps/
install -m 755 $HBB/target/release/teamdesk %{buildroot}/usr/bin/teamdesk
install $HBB/libsciter-gtk.so %{buildroot}/usr/share/teamdesk/libsciter-gtk.so
install $HBB/res/teamdesk.service %{buildroot}/usr/share/teamdesk/files/
install $HBB/res/128x128@2x.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/teamdesk.png
install $HBB/res/scalable.svg %{buildroot}/usr/share/icons/hicolor/scalable/apps/teamdesk.svg
install $HBB/res/teamdesk.desktop %{buildroot}/usr/share/teamdesk/files/
install $HBB/res/teamdesk-link.desktop %{buildroot}/usr/share/teamdesk/files/

%files
/usr/bin/teamdesk
/usr/share/teamdesk/libsciter-gtk.so
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
    rm /usr/share/applications/teamdesk.desktop || true
    rm /usr/share/applications/teamdesk-link.desktop || true
    update-desktop-database
  ;;
  1)
    # for upgrade
  ;;
esac

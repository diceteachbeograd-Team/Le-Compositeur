Name:           le-compositeur
Version:        %{?version}%{!?version:2026.03.10}
Release:        %{?release}%{!?release:1}%{?dist}
Summary:        Le Compositeur dynamic desktop GUI (Rust)

License:        GPL-3.0-or-later
URL:            https://github.com/diceteachbeograd-Team/Le-Compositeur
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  desktop-file-utils
Requires:       mpv
Recommends:     yt-dlp

%description
Le Compositeur renders dynamic wallpapers using local or public image/quote sources,
clock overlays, and desktop backend integration.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release -p wc-cli
cargo build --release -p wc-gui

%install
install -Dpm0755 target/release/wc-cli %{buildroot}%{_bindir}/le-compositeur-cli
install -Dpm0755 target/release/wc-gui %{buildroot}%{_libexecdir}/%{name}/le-compositeur-bin
install -Dpm0755 packaging/linux/le-compositeur-wrapper.sh %{buildroot}%{_bindir}/le-compositeur
ln -s le-compositeur %{buildroot}%{_bindir}/wc-gui
ln -s le-compositeur-cli %{buildroot}%{_bindir}/wc-cli
install -Dpm0644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -Dpm0644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE
install -Dpm0644 assets/icons/le-compositeur.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/le-compositeur.png
install -Dpm0644 assets/quotes/local/local-quotes.md %{buildroot}%{_datadir}/%{name}/quotes/local-quotes.md
install -Dpm0644 packaging/linux/le-compositeur.desktop %{buildroot}%{_datadir}/applications/le-compositeur.desktop
install -Dpm0644 packaging/linux/le-compositeur.metainfo.xml %{buildroot}%{_datadir}/metainfo/le-compositeur.metainfo.xml

%preun
if [ "$1" -eq 0 ]; then
  for d in /home/*; do
    [ -d "$d/.config/autostart" ] || continue
    rm -f "$d/.config/autostart/le-compositeur.desktop" "$d/.config/autostart/wallpaper-composer.desktop" || true
  done
fi

%files
%license %{_licensedir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/README.md
%{_bindir}/wc-cli
%{_bindir}/wc-gui
%{_bindir}/le-compositeur
%{_bindir}/le-compositeur-cli
%{_libexecdir}/%{name}/le-compositeur-bin
%{_datadir}/icons/hicolor/512x512/apps/le-compositeur.png
%{_datadir}/%{name}/quotes/local-quotes.md
%{_datadir}/applications/le-compositeur.desktop
%{_datadir}/metainfo/le-compositeur.metainfo.xml

%changelog
* Tue Mar 10 2026 Le Compositeur Contributors <opensource@example.com> - 2026.03.10-4
- Add single-instance protections for wc-cli run loop and GUI start actions.
- Deduplicate autostart entries and remove legacy wallpaper-composer desktop entry.
- Add uninstall autostart cleanup hook for package remove.
- Update release workflow to set non-draft releases explicitly.

* Mon Mar 09 2026 Le Compositeur Contributors <opensource@example.com> - 2026.03.09-3
- Rename Linux package output to le-compositeur and add le-compositeur binaries.
- Keep wc-cli and wc-gui symlinks for compatibility.

* Mon Mar 09 2026 Le Compositeur Contributors <opensource@example.com> - 2026.03.09-2
- Add fixed 16:9 News size presets in GUI and enforce preset snapping.
- Improve release artifact workflow with explicit OS-targeted package outputs.
- Rebrand user-facing desktop/metainfo/README strings to Le Compositeur.

* Sun Mar 08 2026 Le Compositeur Contributors <opensource@example.com> - 2026.03.08-5
- Add configurable weather/news widget sizes and compact weather/news overlay rendering.
- Add custom camera URL support for News widget with 1.0 FPS cap.

* Sun Mar 08 2026 Le Compositeur Contributors <opensource@example.com> - 2026.03.08-3
- Add weather fallback via wttr.in and cleaner news overlay text for widgets.

* Sun Mar 08 2026 Le Compositeur Contributors <opensource@example.com> - 2026.03.08-2
- Add weather geolocation fallbacks and news preview image overlay support.

* Sun Mar 08 2026 Le Compositeur Contributors <opensource@example.com> - 2026.03.08-1
- Alpha packaging update with GUI binary, desktop entry, icon, and metainfo.

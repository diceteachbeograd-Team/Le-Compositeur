Name:           wallpaper-composer
Version:        1.20260306.1
Release:        1%{?dist}
Summary:        Dynamic Linux wallpaper composer (Rust)

License:        GPL-3.0-or-later
URL:            https://github.com/diceteachbeograd-Team/wallpaper-composer
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  desktop-file-utils

%description
Wallpaper Composer renders dynamic wallpapers using local or public image/quote sources,
clock overlays, and desktop backend integration.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release -p wc-cli
cargo build --release -p wc-gui

%install
install -Dpm0755 target/release/wc-cli %{buildroot}%{_bindir}/wc-cli
install -Dpm0755 target/release/wc-gui %{buildroot}%{_libexecdir}/%{name}/wc-gui-bin
install -Dpm0755 packaging/linux/wc-gui-wrapper.sh %{buildroot}%{_bindir}/wc-gui
install -Dpm0644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -Dpm0644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE
install -Dpm0644 assets/icons/wallpaper-composer.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/wallpaper-composer.png
install -Dpm0644 assets/quotes/local/local-quotes.md %{buildroot}%{_datadir}/wallpaper-composer/quotes/local-quotes.md
install -Dpm0644 packaging/linux/wallpaper-composer.desktop %{buildroot}%{_datadir}/applications/wallpaper-composer.desktop
install -Dpm0644 packaging/linux/wallpaper-composer.metainfo.xml %{buildroot}%{_datadir}/metainfo/wallpaper-composer.metainfo.xml

%files
%license %{_licensedir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/README.md
%{_bindir}/wc-cli
%{_bindir}/wc-gui
%{_libexecdir}/%{name}/wc-gui-bin
%{_datadir}/icons/hicolor/512x512/apps/wallpaper-composer.png
%{_datadir}/wallpaper-composer/quotes/local-quotes.md
%{_datadir}/applications/wallpaper-composer.desktop
%{_datadir}/metainfo/wallpaper-composer.metainfo.xml

%changelog
* Fri Mar 06 2026 Wallpaper Composer Contributors <opensource@example.com> - 1.20260306.1-1
- Alpha packaging update with GUI binary, desktop entry, icon, and metainfo.

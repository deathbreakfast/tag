//! Mount TagRoutes (eager) for Playwright.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use uf_integrations::{
    provide_shell_auth_menu, HostAuthMenu, ShellAppBar, ShellAuthMenu, UnifiedFieldAppBar,
    UnifiedFieldShellLayout,
};
use uf_product::components::ContentContainer;
use uf_product::primitives::{Body1, Flex, FlexAlign, FlexGap, Link, Title3};
use uf_product::{orbital_shell, OrbitalTemplate};

use crate::gate_demos::E2eAuthProvider;
use crate::harness_auth_menu::HarnessAuthMenu;
use crate::picker_demo::TagPickerDemoPage;
use crate::tag_routes_eager::TagRoutesEager;

/// SSR document shell.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    orbital_shell(options, || view! { <App/> })
}

/// Root: harness auth + eager tag routes (same pages as TagRoutes).
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    #[cfg(feature = "ssr")]
    {
        provide_context(crate::e2e_higgs_config());
    }
    provide_shell_auth_menu(|| view! { <HarnessAuthMenu /> });

    view! {
        <OrbitalTemplate>
            <Stylesheet id="leptos" href="/pkg/tag-ui-e2e.css"/>
            <Title text="tag-ui-e2e"/>
            <E2eAuthProvider>
                <Router>
                    <Routes fallback=|| view! { <p>"Not found"</p> }>
                        <Route path=path!("/") view=HomePage/>
                        <Route path=path!("/dev/tag-picker") view=TagPickerDemoPage/>
                        <TagRoutesEager />
                    </Routes>
                </Router>
            </E2eAuthProvider>
        </OrbitalTemplate>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar app_name="Tag e2e".to_string()>
                    <ShellAuthMenu slot:auth_menu>
                        <HostAuthMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ContentContainer max_width="900px" data_testid="tag-e2e-home">
                <Flex vertical=true gap=FlexGap::Medium align=FlexAlign::Start>
                    <Title3>"tag-ui-e2e"</Title3>
                    <Body1>"TagRoutes host for Playwright."</Body1>
                    <Link href="/tag">"Open /tag"</Link>
                </Flex>
            </ContentContainer>
        </UnifiedFieldShellLayout>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    // Eager routes (see tag_routes_eager) — hydrate_body is enough.
    leptos::mount::hydrate_body(App);
    uf_product::hide_boot_loader();
}

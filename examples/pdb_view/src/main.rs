//use ms_pdb::Pdb;

fn main() {
    let app = forms::App::new();

    let main_form = app
        .form_builder()
        .size(1280, 1024)
        .title("PDB Viewer")
        .build();

    // main_form.set_layout(layout);

    main_form.show_modal();
}

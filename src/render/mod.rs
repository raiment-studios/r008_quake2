use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin)
            .add_systems(Startup, setup_fps)
            .add_systems(PostUpdate, fps_update);
    }
}

/// Set up fps counter in the bottom right of the screen
fn setup_fps(mut commands: Commands) {
    use bevy::color::Color;

    commands.spawn((
        TextBundle::from_sections([TextSection::from_style(TextStyle {
            font_size: 20.0,
            color: Color::srgb(0.7, 0.5, 0.1),
            ..default()
        })])
        .with_style(Style {
            left: Val::Px(8.0),
            top: Val::Percent(95.0),
            ..Default::default()
        }),
        FpsText,
    ));
}

#[derive(Component)]
pub struct FpsText;

/// Update the fps each frame
fn fps_update(
    frame_count: Res<bevy::core::FrameCount>,
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    let fps: f64 = match diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        Some(fps) => match fps.smoothed() {
            Some(value) => value,
            _ => 0.0,
        },
        None => 0.0,
    };

    for mut text in &mut query {
        let fps = (fps * 5.0).round() / 5.0;
        text.sections[0].value = format!("{} / {:.0}", frame_count.0, fps);
    }
}

# Bevy Spacesim

![](.github/screenshot.png)

## Description

This project is a space simulation application built using the Bevy game engine, along with the `big_space` and `egui`
libraries.

## Known issues

- Orbit lines are misaligned with their respective bodies.
- No textures

## Compilation and Running

To compile and run the project, use the standard Rust procedures. Ensure you have `cargo` installed.

1. Clone the repository:
    ```sh
    git clone https://github.com/solarliner/bevy-spacesim
    cd bevy-spacesim
    ```

2. Build the project:
    ```sh
    cargo build --features dev
    ```

3. Run the project:
    ```sh
    cargo run --features dev
    ```

## Credits

This project uses data from the [HYG Database](https://github.com/astronexus/HYG-Database). We extend our gratitude to
the creators.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.

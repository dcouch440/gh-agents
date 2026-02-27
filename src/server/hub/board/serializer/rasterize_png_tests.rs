#[cfg(test)]
mod tests {
    use crate::server::hub::board_serializer::CanvasBounds;

    use crate::server::hub::board::serializer::rasterize_png::rasterize_strokes_png;

    fn dummy_bounds() -> CanvasBounds {
        CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 500.0,
            height: 500.0,
        }
    }

    #[test]
    fn empty_strokes_returns_none() {
        let result = rasterize_strokes_png(&[], &dummy_bounds(), 200, 10, 3);
        assert!(result.is_none());
    }

    #[test]
    fn empty_points_returns_none() {
        let strokes: Vec<Vec<[f64; 3]>> = vec![vec![]];
        let result = rasterize_strokes_png(&strokes, &dummy_bounds(), 200, 10, 3);
        assert!(result.is_none());
    }

    #[test]
    fn single_point_produces_valid_png() {
        let strokes = vec![vec![[100.0, 100.0, 0.5]]];
        let result = rasterize_strokes_png(&strokes, &dummy_bounds(), 200, 10, 3);
        assert!(result.is_some());

        // Verify it's valid base64 that decodes to a PNG
        let b64 = result.unwrap();
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64)
            .expect("valid base64");
        // PNG magic bytes
        assert_eq!(&bytes[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn simple_line_produces_expected_dimensions() {
        // Horizontal line 400px wide, 0px tall → bbox_w=400, bbox_h=0
        // Longest side is 400, scale to max_side=200 minus padding
        let strokes = vec![vec![[0.0, 100.0, 0.5], [400.0, 100.0, 0.5]]];
        let result = rasterize_strokes_png(&strokes, &dummy_bounds(), 200, 10, 3);
        assert!(result.is_some());

        let b64 = result.unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64).unwrap();

        // Decode the PNG to verify dimensions
        let img = image::load_from_memory(&bytes).expect("valid PNG");
        // Width should be around 200 (max_side)
        assert!(img.width() <= 200);
        assert!(img.width() >= 180); // 180px usable + padding
    }

    #[test]
    fn multiple_strokes_rasterize() {
        let strokes = vec![
            vec![
                [10.0, 10.0, 0.5],
                [50.0, 10.0, 0.5],
                [50.0, 50.0, 0.5],
            ],
            vec![[100.0, 100.0, 0.5], [200.0, 200.0, 0.5]],
        ];
        let result = rasterize_strokes_png(&strokes, &dummy_bounds(), 400, 10, 3);
        assert!(result.is_some());

        let b64 = result.unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64).unwrap();
        let img = image::load_from_memory(&bytes).expect("valid PNG");
        // Should have some black pixels (strokes drawn)
        let gray = img.to_luma8();
        let has_black = gray.pixels().any(|p| p.0[0] < 128);
        assert!(has_black, "image should contain black stroke pixels");
    }

    #[test]
    fn scaling_preserves_aspect_ratio() {
        // Square bounding box (200x200 strokes) → should produce roughly square output
        let strokes = vec![vec![
            [100.0, 100.0, 0.5],
            [300.0, 100.0, 0.5],
            [300.0, 300.0, 0.5],
            [100.0, 300.0, 0.5],
            [100.0, 100.0, 0.5],
        ]];
        let result = rasterize_strokes_png(&strokes, &dummy_bounds(), 400, 10, 3);
        assert!(result.is_some());

        let b64 = result.unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64).unwrap();
        let img = image::load_from_memory(&bytes).expect("valid PNG");
        // Should be roughly square — width and height within 5% of each other
        let ratio = img.width() as f64 / img.height() as f64;
        assert!(
            (0.95..=1.05).contains(&ratio),
            "expected square-ish image, got {}x{} (ratio: {ratio:.2})",
            img.width(),
            img.height()
        );
    }

    #[test]
    fn large_strokes_scale_down() {
        // Very large canvas strokes (2000x1000) should scale to fit max_side=768
        let strokes = vec![vec![
            [0.0, 0.0, 0.5],
            [2000.0, 0.0, 0.5],
            [2000.0, 1000.0, 0.5],
            [0.0, 1000.0, 0.5],
        ]];
        let result = rasterize_strokes_png(&strokes, &dummy_bounds(), 768, 10, 3);
        assert!(result.is_some());

        let b64 = result.unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64).unwrap();
        let img = image::load_from_memory(&bytes).expect("valid PNG");
        assert!(img.width() <= 768, "width {} should be <= 768", img.width());
        assert!(
            img.height() <= 768,
            "height {} should be <= 768",
            img.height()
        );
    }

    #[test]
    fn pressure_variation_produces_valid_output() {
        // Stroke with varying pressure should still produce valid PNG
        let strokes = vec![vec![
            [0.0, 0.0, 0.1],
            [50.0, 0.0, 0.3],
            [100.0, 0.0, 0.7],
            [150.0, 0.0, 0.9],
            [200.0, 0.0, 0.5],
        ]];
        let result = rasterize_strokes_png(&strokes, &dummy_bounds(), 400, 10, 5);
        assert!(result.is_some());

        let b64 = result.unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64).unwrap();
        assert_eq!(&bytes[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}

class_name FakeWorldTest
extends GdUnitTestSuite


func test_generate_palette_from_image() -> void:
	# 1. Instantiate your Rust GDExtension class
	var fake_world = auto_free(FakeWorld.new())

	# 2. Create a dummy source image to pass to the function
	var source_image = Image.create(64, 64, false, Image.FORMAT_RGBA8)
	source_image.fill(Color.HOT_PINK)  # Fill it with something just to be safe

	# 3. Call the Rust function
	var palette_tex: Texture2D = fake_world.generate_palette_from_image(source_image)

	# 4. Assert the function returned a valid object
	assert_object(palette_tex).is_not_null()
	assert_bool(palette_tex is Texture2D).is_true()

	# 5. Assert the dimensions match the Rust implementation (16x1)
	assert_int(palette_tex.get_width()).is_equal(16)
	assert_int(palette_tex.get_height()).is_equal(1)

	# 6. Extract the underlying image and assert the format is RGBA8
	var palette_image: Image = palette_tex.get_image()
	assert_object(palette_image).is_not_null()
	assert_int(palette_image.get_format()).is_equal(Image.FORMAT_RGBA8)

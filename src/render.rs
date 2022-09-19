use webrender::{
    api::{
        units::{LayoutPoint, LayoutRect, LayoutSize},
        *,
    },
    RenderApi, Transaction,
};

pub fn render(
    builder: &mut DisplayListBuilder,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    epoch: Epoch,
    api: &mut RenderApi,
    layout_size: LayoutSize,
) {
    let mut txn = Transaction::new();
    builder.begin();

    {
        //TODO render node
        let content_bounds = units::LayoutRect::from_size(layout_size);
        let root_space_and_clip = SpaceAndClipInfo::root_scroll(pipeline_id);
        let spatial_id = root_space_and_clip.spatial_id;

        builder.push_simple_stacking_context(
            content_bounds.min,
            spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        let clip_chain_id = builder.define_clip_chain(None, []);

        builder.push_rect(
            &CommonItemProperties::new(
                LayoutRect::from_origin_and_size(
                    LayoutPoint::new(300 as f32, 300 as f32),
                    LayoutSize::new(100 as f32, 100 as f32),
                ),
                SpaceAndClipInfo {
                    spatial_id,
                    clip_chain_id,
                },
            ),
            LayoutRect::from_origin_and_size(
                LayoutPoint::new(100 as f32, 100 as f32),
                LayoutSize::new(500 as f32, 500 as f32),
            ),
            ColorF::new(0.0, 1.0, 1.0, 1.0),
        );

        builder.pop_stacking_context();
    }

    txn.set_display_list(
        epoch,
        Some(ColorF::new(0., 0., 0., 1.0)),
        layout_size,
        builder.end(),
    );
    txn.set_root_pipeline(pipeline_id);
    txn.generate_frame(0, RenderReasons::empty());
    api.send_transaction(document_id, txn);
}

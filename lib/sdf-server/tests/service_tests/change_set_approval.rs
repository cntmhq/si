use std::collections::HashMap;
use std::collections::HashSet;

use dal::approval_requirement::ApprovalRequirement;
use dal::approval_requirement::ApprovalRequirementApprover;
use dal::change_set::approval::ChangeSetApproval;
use dal::diagram::view::View;
use dal::Component;
use dal::ComponentType;
use dal::DalContext;
use dal::HistoryActor;
use dal::SchemaVariant;
use dal_test::eyre;
use dal_test::helpers::create_component_for_default_schema_name;
use dal_test::helpers::create_schema;
use dal_test::prelude::ChangeSetTestHelpers;
use dal_test::sdf_test;
use dal_test::Result;
use indoc::indoc;
use permissions::ObjectType;
use permissions::Relation;
use permissions::RelationBuilder;
use pretty_assertions_sorted::assert_eq;
use sdf_server::dal_wrapper;
use si_data_spicedb::SpiceDbClient;
use si_events::workspace_snapshot::EntityKind;
use si_events::ChangeSetApprovalStatus;
use si_frontend_types::RawGeometry;

// FIXME(nick,jacob): this must happen in the "sdf_test"'s equivalent to global setup, but not in
// dal tests. This also should _really_ reflect the "schema.zed" file that production uses.
async fn write_schema(client: &mut SpiceDbClient) -> Result<()> {
    let schema = indoc! {"
        definition user {}

        definition workspace {
          relation approver: user
          relation owner: user
          permission approve = approver+owner
          permission manage = owner
        }
    "};
    client.write_schema(schema).await?;
    Ok(())
}

// NOTE(nick): this is an integration test and not a service test, but given that "sdf_test" is in
// a weird, unused place at the time of writing, this test will live here.
#[sdf_test]
async fn single_user_relation_existence_and_checksum_validility_permutations(
    ctx: &mut DalContext,
    spicedb_client: SpiceDbClient,
) -> Result<()> {
    let mut spicedb_client = spicedb_client;

    // FIXME(nick,jacob): see the comment attached to this function.
    write_schema(&mut spicedb_client).await?;

    let workspace_id = ctx.workspace_pk()?;
    let user_id = match ctx.history_actor() {
        HistoryActor::SystemInit => return Err(eyre!("invalid user")),
        HistoryActor::User(user_id) => *user_id,
    };

    // Scenario 1: create the variant without the relation.
    let (entity_id, first_approval_id) = {
        let schema = create_schema(ctx).await?;
        let (variant, _) = SchemaVariant::new(
            ctx,
            schema.id(),
            "ringo starr",
            "ringo".to_string(),
            "beatles",
            "#FFFFFF",
            ComponentType::Component,
            None,
            None,
            None,
            false,
        )
        .await?;
        let entity_id: si_id::EntityId = {
            let raw_id: si_id::ulid::Ulid = variant.id().into();
            raw_id.into()
        };
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let approving_ids_with_hashes =
            dal_wrapper::change_set::determine_approving_ids_with_hashes(ctx, &mut spicedb_client)
                .await?;
        let first_approval = ChangeSetApproval::new(
            ctx,
            ChangeSetApprovalStatus::Approved,
            approving_ids_with_hashes,
        )
        .await?;
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;

        assert_eq!(
            vec![si_frontend_types::ChangeSetApproval {
                id: first_approval.id(),
                user_id,
                status: ChangeSetApprovalStatus::Approved,
                is_valid: true
            }], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![si_frontend_types::ChangeSetApprovalRequirement {
                entity_id,
                entity_kind: EntityKind::SchemaVariant,
                required_count: 1,
                is_satisfied: false,
                applicable_approval_ids: Vec::new(),
                approver_groups: HashMap::from_iter(vec![(
                    format!("workspace#{workspace_id}#approve"),
                    Vec::new(),
                )]),
                approver_individuals: Vec::new(),
            }], // expected
            frontend_requirements // actual
        );

        (entity_id, first_approval.id())
    };

    // Scenario 2: create the relation and do not create another approval. The approval should be
    // invalid because its checksum was based on the root node.
    let relation = {
        let relation = RelationBuilder::new()
            .object(ObjectType::Workspace, workspace_id)
            .relation(Relation::Approver)
            .subject(ObjectType::User, user_id);
        relation.create(&mut spicedb_client).await?;

        let (frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;

        assert_eq!(
            vec![si_frontend_types::ChangeSetApproval {
                id: first_approval_id,
                user_id,
                status: ChangeSetApprovalStatus::Approved,
                is_valid: false
            }], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![si_frontend_types::ChangeSetApprovalRequirement {
                entity_id,
                entity_kind: EntityKind::SchemaVariant,
                required_count: 1,
                is_satisfied: false,
                applicable_approval_ids: vec![first_approval_id],
                approver_groups: HashMap::from_iter(vec![(
                    format!("workspace#{workspace_id}#approve"),
                    vec![user_id],
                )]),
                approver_individuals: Vec::new(),
            }], // expected
            frontend_requirements // actual
        );

        relation
    };

    // Scenario 3: create an approval with our relation intact. Our new approval should satisfy the
    // requirement.
    let second_approval_id = {
        let approving_ids_with_hashes =
            dal_wrapper::change_set::determine_approving_ids_with_hashes(ctx, &mut spicedb_client)
                .await?;
        let second_approval = ChangeSetApproval::new(
            ctx,
            ChangeSetApprovalStatus::Approved,
            approving_ids_with_hashes,
        )
        .await?;
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (mut frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_latest_approvals.sort_by_key(|a| a.id);
        frontend_requirements
            .iter_mut()
            .for_each(|r| r.applicable_approval_ids.sort());

        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApproval {
                    id: first_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: false
                },
                si_frontend_types::ChangeSetApproval {
                    id: second_approval.id(),
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: true
                }
            ], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![si_frontend_types::ChangeSetApprovalRequirement {
                entity_id,
                entity_kind: EntityKind::SchemaVariant,
                required_count: 1,
                is_satisfied: true,
                applicable_approval_ids: vec![first_approval_id, second_approval.id()],
                approver_groups: HashMap::from_iter(vec![(
                    format!("workspace#{workspace_id}#approve"),
                    vec![user_id]
                )]),
                approver_individuals: Vec::new(),
            }], // expected
            frontend_requirements // actual
        );

        second_approval.id()
    };

    // Scenario 4: delete the relation and do not create an approval. The newest approval for the
    // original permissions should still have a valid checksum, but it should not be applicable for
    // the requirement.
    {
        relation.delete(&mut spicedb_client).await?;

        let (mut frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_latest_approvals.sort_by_key(|a| a.id);

        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApproval {
                    id: first_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: true
                },
                si_frontend_types::ChangeSetApproval {
                    id: second_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: false
                }
            ], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![si_frontend_types::ChangeSetApprovalRequirement {
                entity_id,
                entity_kind: EntityKind::SchemaVariant,
                required_count: 1,
                is_satisfied: false,
                applicable_approval_ids: Vec::new(),
                approver_groups: HashMap::from_iter(vec![(
                    format!("workspace#{workspace_id}#approve"),
                    Vec::new()
                )]),
                approver_individuals: Vec::new(),
            }], // expected
            frontend_requirements // actual
        );
    }

    // Scenario 5: re-create the relation, but do not create an approval. The newest approval
    // should still have a valid checksum and it should satisfy the requirement, once again.
    {
        relation.create(&mut spicedb_client).await?;

        let (mut frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_latest_approvals.sort_by_key(|a| a.id);
        frontend_requirements
            .iter_mut()
            .for_each(|r| r.applicable_approval_ids.sort());

        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApproval {
                    id: first_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: false
                },
                si_frontend_types::ChangeSetApproval {
                    id: second_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: true
                }
            ], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![si_frontend_types::ChangeSetApprovalRequirement {
                entity_id,
                entity_kind: EntityKind::SchemaVariant,
                required_count: 1,
                is_satisfied: true,
                applicable_approval_ids: vec![first_approval_id, second_approval_id],
                approver_groups: HashMap::from_iter(vec![(
                    format!("workspace#{workspace_id}#approve"),
                    vec![user_id]
                )]),
                approver_individuals: Vec::new(),
            }], // expected
            frontend_requirements // actual
        );
    }

    Ok(())
}

// NOTE(nick): this is an integration test and not a service test, but given that "sdf_test" is in
// a weird, unused place at the time of writing, this test will live here.
#[sdf_test]
async fn individual_approver_for_view(
    ctx: &mut DalContext,
    spicedb_client: SpiceDbClient,
) -> Result<()> {
    let mut spicedb_client = spicedb_client;

    // FIXME(nick,jacob): see the comment attached to this function.
    write_schema(&mut spicedb_client).await?;

    // Cache the IDs we need.
    let workspace_id = ctx.workspace_pk()?;
    let user_id = match ctx.history_actor() {
        HistoryActor::SystemInit => return Err(eyre!("invalid user")),
        HistoryActor::User(user_id) => *user_id,
    };
    let view_id = View::get_id_for_default(ctx).await?;

    // Scenario 1: see all approvals and requirements with an "empty" workspace.
    {
        let (frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;

        assert!(frontend_latest_approvals.is_empty());
        assert!(frontend_requirements.is_empty());
    }

    // Scenario 2: add the approval requirement to the default view with ourself as the individual approver.
    let (
        view_entity_id,
        approval_requirement_definition_entity_id,
        approval_requirement_definition_id,
    ) = {
        let approval_requirement_definition_id = ApprovalRequirement::new_definition(
            ctx,
            view_id,
            1,
            HashSet::from([ApprovalRequirementApprover::User(user_id)]),
        )
        .await?;
        let (view_entity_id, approval_requirement_definition_entity_id) = (
            view_id.into_inner().into(),
            approval_requirement_definition_id.into_inner().into(),
        );
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert!(frontend_latest_approvals.is_empty());
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: view_entity_id,
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::new(),
                    approver_individuals: vec![user_id]
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: approval_requirement_definition_entity_id,
                    entity_kind: EntityKind::ApprovalRequirementDefinition,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        Vec::new()
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );

        (
            view_entity_id,
            approval_requirement_definition_entity_id,
            approval_requirement_definition_id,
        )
    };

    // Scenario 3: create an approval that will satisfy the view requirement, but not the
    // definition requirement.
    let first_approval_id = {
        let first_approval_id = {
            let approving_ids_with_hashes =
                dal_wrapper::change_set::determine_approving_ids_with_hashes(
                    ctx,
                    &mut spicedb_client,
                )
                .await?;
            let first_approval = ChangeSetApproval::new(
                ctx,
                ChangeSetApprovalStatus::Approved,
                approving_ids_with_hashes,
            )
            .await?;
            first_approval.id()
        };
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert_eq!(
            vec![si_frontend_types::ChangeSetApproval {
                id: first_approval_id,
                user_id,
                status: ChangeSetApprovalStatus::Approved,
                is_valid: true,
            }], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: view_entity_id,
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: true,
                    applicable_approval_ids: vec![first_approval_id],
                    approver_groups: HashMap::new(),
                    approver_individuals: vec![user_id]
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: approval_requirement_definition_entity_id,
                    entity_kind: EntityKind::ApprovalRequirementDefinition,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        Vec::new()
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );

        first_approval_id
    };

    // Scenario 4: add ourself as a workspace approver. The original approval should no longer
    // satisfy the view requirement because the IDs we are approving has changed. It would also
    // not satisfy the definition requirement for the same reason.
    {
        let relation = RelationBuilder::new()
            .object(ObjectType::Workspace, workspace_id)
            .relation(Relation::Approver)
            .subject(ObjectType::User, user_id);
        relation.create(&mut spicedb_client).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert_eq!(
            vec![si_frontend_types::ChangeSetApproval {
                id: first_approval_id,
                user_id,
                status: ChangeSetApprovalStatus::Approved,
                is_valid: false,
            }], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: view_entity_id,
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: vec![first_approval_id],
                    approver_groups: HashMap::new(),
                    approver_individuals: vec![user_id]
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: approval_requirement_definition_entity_id,
                    entity_kind: EntityKind::ApprovalRequirementDefinition,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: vec![first_approval_id],
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );
    };

    // Scenario 5: create a rejection.
    {
        let second_approval_id = {
            let approving_ids_with_hashes =
                dal_wrapper::change_set::determine_approving_ids_with_hashes(
                    ctx,
                    &mut spicedb_client,
                )
                .await?;
            let second_approval = ChangeSetApproval::new(
                ctx,
                ChangeSetApprovalStatus::Rejected,
                approving_ids_with_hashes,
            )
            .await?;
            second_approval.id()
        };
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (mut frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_latest_approvals.sort_by_key(|a| a.id);
        frontend_requirements.sort_by_key(|r| r.entity_id);
        frontend_requirements
            .iter_mut()
            .for_each(|r| r.applicable_approval_ids.sort());

        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApproval {
                    id: first_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: false,
                },
                si_frontend_types::ChangeSetApproval {
                    id: second_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Rejected,
                    is_valid: true,
                }
            ], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: view_entity_id,
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: vec![first_approval_id, second_approval_id],
                    approver_groups: HashMap::new(),
                    approver_individuals: vec![user_id]
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: approval_requirement_definition_entity_id,
                    entity_kind: EntityKind::ApprovalRequirementDefinition,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: vec![first_approval_id, second_approval_id],
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );
    }

    // Scenario 6: create an approval that will satisfy both requirements.
    {
        let third_approval_id = {
            let approving_ids_with_hashes =
                dal_wrapper::change_set::determine_approving_ids_with_hashes(
                    ctx,
                    &mut spicedb_client,
                )
                .await?;
            let third_approval = ChangeSetApproval::new(
                ctx,
                ChangeSetApprovalStatus::Approved,
                approving_ids_with_hashes,
            )
            .await?;
            third_approval.id()
        };
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (mut frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_latest_approvals.sort_by_key(|a| a.id);
        frontend_requirements.sort_by_key(|r| r.entity_id);
        frontend_requirements
            .iter_mut()
            .for_each(|r| r.applicable_approval_ids.sort());

        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApproval {
                    id: first_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: false,
                },
                si_frontend_types::ChangeSetApproval {
                    id: third_approval_id,
                    user_id,
                    status: ChangeSetApprovalStatus::Approved,
                    is_valid: true,
                }
            ], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: view_entity_id,
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: true,
                    applicable_approval_ids: vec![first_approval_id, third_approval_id],
                    approver_groups: HashMap::new(),
                    approver_individuals: vec![user_id]
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: approval_requirement_definition_entity_id,
                    entity_kind: EntityKind::ApprovalRequirementDefinition,
                    required_count: 1,
                    is_satisfied: true,
                    applicable_approval_ids: vec![first_approval_id, third_approval_id],
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );
    }

    // Scenario 7: apply the change set, create a new change set and observe that no approvals nor requirements exist.
    {
        ChangeSetTestHelpers::apply_change_set_to_base(ctx).await?;
        ChangeSetTestHelpers::fork_from_head_change_set(ctx).await?;

        let (frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;

        assert!(frontend_latest_approvals.is_empty());
        assert!(frontend_requirements.is_empty());
    }

    // Scenario 8: remove the definiton from the view.
    {
        ApprovalRequirement::remove_definition(ctx, approval_requirement_definition_id).await?;
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert!(frontend_latest_approvals.is_empty());
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: view_entity_id,
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: approval_requirement_definition_entity_id,
                    entity_kind: EntityKind::ApprovalRequirementDefinition,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                },
            ], // expected
            frontend_requirements // actual
        );
    }

    // Scenario 9: reject the removal.
    {
        let fourth_approval_id = {
            let approving_ids_with_hashes =
                dal_wrapper::change_set::determine_approving_ids_with_hashes(
                    ctx,
                    &mut spicedb_client,
                )
                .await?;
            let fourth_approval = ChangeSetApproval::new(
                ctx,
                ChangeSetApprovalStatus::Rejected,
                approving_ids_with_hashes,
            )
            .await?;
            fourth_approval.id()
        };
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert_eq!(
            vec![si_frontend_types::ChangeSetApproval {
                id: fourth_approval_id,
                user_id,
                status: ChangeSetApprovalStatus::Rejected,
                is_valid: true,
            },], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: view_entity_id,
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: vec![fourth_approval_id],
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: approval_requirement_definition_entity_id,
                    entity_kind: EntityKind::ApprovalRequirementDefinition,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: vec![fourth_approval_id],
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );
    }

    // Scenario 10: approve the removal.
    {
        let fifth_approval_id = {
            let approving_ids_with_hashes =
                dal_wrapper::change_set::determine_approving_ids_with_hashes(
                    ctx,
                    &mut spicedb_client,
                )
                .await?;
            let fifth_approval = ChangeSetApproval::new(
                ctx,
                ChangeSetApprovalStatus::Approved,
                approving_ids_with_hashes,
            )
            .await?;
            fifth_approval.id()
        };
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert_eq!(
            vec![si_frontend_types::ChangeSetApproval {
                id: fifth_approval_id,
                user_id,
                status: ChangeSetApprovalStatus::Approved,
                is_valid: true,
            },], // expected
            frontend_latest_approvals // actual
        );
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: view_entity_id,
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: true,
                    applicable_approval_ids: vec![fifth_approval_id],
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: approval_requirement_definition_entity_id,
                    entity_kind: EntityKind::ApprovalRequirementDefinition,
                    required_count: 1,
                    is_satisfied: true,
                    applicable_approval_ids: vec![fifth_approval_id],
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        vec![user_id],
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );
    }

    Ok(())
}

// NOTE(nick): this is an integration test and not a service test, but given that "sdf_test" is in
// a weird, unused place at the time of writing, this test will live here.
#[sdf_test]
async fn one_component_in_two_views(
    ctx: &mut DalContext,
    spicedb_client: SpiceDbClient,
) -> Result<()> {
    let mut spicedb_client = spicedb_client;

    // FIXME(nick,jacob): see the comment attached to this function.
    write_schema(&mut spicedb_client).await?;

    // Cache the IDs we need.
    let workspace_id = ctx.workspace_pk()?;
    let user_id = match ctx.history_actor() {
        HistoryActor::SystemInit => return Err(eyre!("invalid user")),
        HistoryActor::User(user_id) => *user_id,
    };

    // Create a view with a requirement and then commit.
    let todd_view = View::new(ctx, "toddhoward").await?;
    let todd_view_id = todd_view.id();
    ApprovalRequirement::new_definition(
        ctx,
        todd_view_id,
        1,
        HashSet::from([ApprovalRequirementApprover::User(user_id)]),
    )
    .await?;
    ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

    // Create a second view with another requirement and then commit.
    let sven_view = View::new(ctx, "svenvincke").await?;
    let sven_view_id = sven_view.id();
    ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

    // Scenario 1: apply to HEAD and create a new change set.
    {
        ChangeSetTestHelpers::apply_change_set_to_base(ctx).await?;
        ChangeSetTestHelpers::fork_from_head_change_set(ctx).await?;

        let (frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;

        assert!(frontend_latest_approvals.is_empty());
        assert!(frontend_requirements.is_empty());
    }

    // Scenario 2: create a component in our new views.
    let component = {
        let component = create_component_for_default_schema_name(
            ctx,
            "starfield",
            "shattered space",
            todd_view_id,
        )
        .await?;
        Component::add_to_view(ctx, component.id(), sven_view_id, RawGeometry::default()).await?;
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert!(frontend_latest_approvals.is_empty());
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: todd_view_id.into_inner().into(),
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::new(),
                    approver_individuals: vec![user_id],
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: sven_view_id.into_inner().into(),
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        Vec::new()
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );

        component
    };

    // Scenario 3: apply to HEAD and create a new change set (skip approvals).
    {
        ChangeSetTestHelpers::apply_change_set_to_base(ctx).await?;
        ChangeSetTestHelpers::fork_from_head_change_set(ctx).await?;

        let (frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;

        assert!(frontend_latest_approvals.is_empty());
        assert!(frontend_requirements.is_empty());
    }

    // Scenario 4: modify the component in our new view.
    {
        component.set_name(ctx, "bg3 patch 8 plz").await?;
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert!(frontend_latest_approvals.is_empty());
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: todd_view_id.into_inner().into(),
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::new(),
                    approver_individuals: vec![user_id],
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: sven_view_id.into_inner().into(),
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        Vec::new()
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );
    }

    // Scenario 5: apply to HEAD and create a new change set (skip approvals).
    {
        ChangeSetTestHelpers::apply_change_set_to_base(ctx).await?;
        ChangeSetTestHelpers::fork_from_head_change_set(ctx).await?;

        let (frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;

        assert!(frontend_latest_approvals.is_empty());
        assert!(frontend_requirements.is_empty());
    }

    // Scenario 6: delete the component in our new view.
    {
        component.delete(ctx).await?;
        ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx).await?;

        let (frontend_latest_approvals, mut frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;
        frontend_requirements.sort_by_key(|r| r.entity_id);

        assert!(frontend_latest_approvals.is_empty());
        assert_eq!(
            vec![
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: todd_view_id.into_inner().into(),
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::new(),
                    approver_individuals: vec![user_id],
                },
                si_frontend_types::ChangeSetApprovalRequirement {
                    entity_id: sven_view_id.into_inner().into(),
                    entity_kind: EntityKind::View,
                    required_count: 1,
                    is_satisfied: false,
                    applicable_approval_ids: Vec::new(),
                    approver_groups: HashMap::from_iter(vec![(
                        format!("workspace#{workspace_id}#approve"),
                        Vec::new()
                    )]),
                    approver_individuals: Vec::new(),
                }
            ], // expected
            frontend_requirements // actual
        );
    }

    // Scenario 7: apply to HEAD and create a new change set (skip approvals).
    {
        ChangeSetTestHelpers::apply_change_set_to_base(ctx).await?;
        ChangeSetTestHelpers::fork_from_head_change_set(ctx).await?;

        let (frontend_latest_approvals, frontend_requirements) =
            dal_wrapper::change_set::status(ctx, &mut spicedb_client).await?;

        assert!(frontend_latest_approvals.is_empty());
        assert!(frontend_requirements.is_empty());
    }

    Ok(())
}

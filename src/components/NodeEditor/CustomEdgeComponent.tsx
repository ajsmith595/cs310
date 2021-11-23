import React from 'react';
import { getMarkerEnd } from 'react-flow-renderer';
import { Position } from '../../classes/Node';
import { PipeableType, PropertyType } from '../../classes/NodeRegistration';

function bezierPoint(t: number, p1: Position, p2: Position, p3: Position, p4: Position) {

    let firstTerm = p1.multiply(Math.pow(1 - t, 3));
    let secondTerm = p2.multiply(3 * t * Math.pow(1 - t, 2));
    let thirdTerm = p3.multiply(3 * t * t * (1 - t));
    let fourthTerm = p4.multiply(t * t * t);
    return firstTerm.add(secondTerm).add(thirdTerm).add(fourthTerm);
}

function subBezier(t0: number, t1: number, p1: Position, p2: Position, p3: Position, p4: Position) {
    let u0 = 1 - t0;
    let u1 = 1 - t1;

    let qa = p1.multiply(u0 * u0).add(p2.multiply(2 * t0 * u0)).add(p3.multiply(t0 * t0));
    let qb = p1.multiply(u1 * u1).add(p2.multiply(2 * t1 * u1)).add(p3.multiply(t1 * t1));
    let qc = p2.multiply(u0 * u0).add(p3.multiply(2 * t0 * u0)).add(p4.multiply(t0 * t0));
    let qd = p2.multiply(u1 * u1).add(p3.multiply(2 * t1 * u1)).add(p4.multiply(t1 * t1));

    let a = qa.multiply(u0).add(qc.multiply(t0));
    let b = qa.multiply(u1).add(qc.multiply(t1));
    let c = qb.multiply(u0).add(qd.multiply(t0));
    let d = qb.multiply(u1).add(qd.multiply(t1));

    return {
        p1: a,
        p2: b,
        p3: c,
        p4: d
    }
}

function getBezierPathO(params: { p1: Position, p2: Position, p3: Position, p4: Position }) {
    return getBezierPath(params.p1, params.p2, params.p3, params.p4);
}
function getBezierPath(p1: Position, p2: Position, p3: Position, p4: Position) {
    return `M ${p1.x} ${p1.y} C ${p2.x} ${p2.y}, ${p3.x} ${p3.y}, ${p4.x} ${p4.y}`;
}


interface EdgeData {
    sourceType: Array<PropertyType>,
    targetType: Array<PropertyType>,
}
export default function CustomEdgeComponent({
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    style = {},
    data,
    arrowHeadType,
    markerEndId,
}) {

    let edge_data: EdgeData = data;

    let controlX = (sourceX + targetX) / 2;

    let p1 = new Position(sourceX, sourceY);
    let p2 = new Position(controlX, sourceY);
    let p3 = new Position(controlX, targetY);
    let p4 = new Position(targetX, targetY);


    let dist = Math.sqrt(Math.pow(sourceX - targetX, 2) + Math.pow(sourceY - targetY, 2));

    const markerEnd = getMarkerEnd(arrowHeadType, markerEndId);


    let conversion_needed = true;
    let source_type = null;
    let target_type = null;
    if (edge_data.sourceType.length != 1) {
        conversion_needed = false;
    } else {
        source_type = edge_data.sourceType[0].getPipeableType();
        for (let t of edge_data.targetType) {
            let target_type = t.getPipeableType();
            if (target_type == source_type) {
                conversion_needed = false;
                break;
            }
        }
        if (conversion_needed) {
            let priority: Array<PipeableType> = [PipeableType.Container, PipeableType.Video, PipeableType.Audio, PipeableType.Subtitle];
            let i = -1;
            let source_index = priority.indexOf(source_type);
            for (let t of edge_data.targetType) {
                let target_type = t.getPipeableType();
                let index = priority.indexOf(target_type);
                if (index > i && source_index >= index) {
                    i = index;
                }
            }
            if (i != -1) {
                target_type = priority[i];
            }
        }
    }

    if (!conversion_needed || dist < 50) {
        const edgePath1 = getBezierPath(p1, p2, p3, p4);
        style = style || {};



        return (
            <path id={id} style={style} className="stroke-current react-flow__edge-path" d={edgePath1} markerEnd={markerEnd} />
        );
    }






    // map from 0 -> 300
    let delta = Math.min(Math.max(((300 - dist) / 300), 0), 1) * 0.3;



    let pathToMid = subBezier(0, 0.5 - delta, p1, p2, p3, p4);
    let mid = subBezier(0.5 - delta, 0.5 + delta, p1, p2, p3, p4);
    let midToEnd = subBezier(0.5 + delta, 1, p1, p2, p3, p4);

    const edgePath1 = getBezierPathO(pathToMid);
    const edgePath2 = getBezierPathO(mid);
    const edgePath3 = getBezierPathO(midToEnd);

    return (
        <>
            <path id={id} style={style} className="react-flow__edge-path" d={edgePath1} markerEnd={markerEnd} />
            <path id={id} style={style} className="react-flow__edge-path" d={edgePath2} markerEnd={markerEnd} />
            <path id={id} style={style} className="react-flow__edge-path" d={edgePath3} markerEnd={markerEnd} />
            {/* <text>
                <textPath href={`#${id}`} style={{ fontSize: '12px' }} startOffset="50%" textAnchor="middle">
                    {data.text}
                </textPath>
            </text> */}
        </>
    );
}
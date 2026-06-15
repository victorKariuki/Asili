leta matumizi
leta hisabati


kazi manhattan(x1: Namba, y1: Namba, x2: Namba, y2: Namba) -> Namba {
    rejesha absolute(x1 - x2) + absolute(y1 - y2)
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    chapisha("A* Algorithm Example")

    # 0 = walkable, 1 = obstacle
    weka grid = [
        [0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 1.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, 0.0]
    ]

    weka goalx = 4.0
    weka goaly = 4.0

    weka startpos: Jozi<Namba, Namba> = jozi(0.0, 0.0)
    weka openlist = [startpos]

    weka gcosts: Kamusi<Neno, Namba> = {}
    gcosts["0,0"] = 0.0

    weka parents: Kamusi<Neno, Neno> = {}
    weka found = si_kweli

    wakati openlist.urefu() > 0.0 {
        # Find node with lowest f_cost
        weka bestidx = 0.0
        weka minf = 1000000.0

        weka i = 0.0
        wakati i < openlist.urefu() {
            weka pos = openlist[i]?
            weka px = pos.kwanza()
            weka py = pos.pili()

            weka key = (px kama Neno) + "," + (py kama Neno)
            weka g = gcosts.pata(key).angu(1000000.0)
            weka h = manhattan(px, py, goalx, goaly)
            weka f = g + h

            ikiwa f < minf {
                minf = f
                bestidx = i
            }
            i = i + 1.0
        }

        weka current = openlist.ondoa(bestidx).angu(jozi(0.0, 0.0))
        weka cx = current.kwanza()
        weka cy = current.pili()

        ikiwa cx == goalx && cy == goaly {
            found = kweli
            vunja
        }

        # Neighbor offsets: up, down, right, left
        weka dx = [0.0, 0.0, 1.0, -1.0]
        weka dy = [1.0, -1.0, 0.0, 0.0]

        weka j = 0.0
        wakati j < 4.0 {
            weka nx = cx + dx[j]?
            weka ny = cy + dy[j]?

            ikiwa nx >= 0.0 && nx < 5.0 && ny >= 0.0 && ny < 5.0 {
                weka row = grid[ny]?
                weka val = row[nx]?

                ikiwa val == 0.0 {
                    weka nkey = (nx kama Neno) + "," + (ny kama Neno)
                    weka ckey = (cx kama Neno) + "," + (cy kama Neno)

                    weka g_cx_cy = gcosts.pata(ckey.clona()).angu(0.0)
                    weka newg = g_cx_cy + 1.0
                    weka oldg = gcosts.pata(nkey.clona()).angu(1000000.0)

                    ikiwa newg < oldg {
                        gcosts[nkey.clona()] = newg
                        parents[nkey] = ckey
                        openlist.ongeza(jozi(nx, ny))
                    }
                }
            }
            j = j + 1.0
        }
    }

    ikiwa found {
        chapisha("Path found!")
        weka curr = (goalx kama Neno) + "," + (goaly kama Neno)
        wakati curr != "0,0" {
            chapisha(curr.clona())
            curr = parents.pata(curr).angu("")
        }
        chapisha("0,0")
    } vinginevyo {
        chapisha("No path found.")
    }
}
